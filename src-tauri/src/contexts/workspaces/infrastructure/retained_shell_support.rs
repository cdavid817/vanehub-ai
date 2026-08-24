//! The small adapters a retained Shell registry needs: where it may open, what time it is, what to
//! call things, and how a subscriber hears about a change.

use crate::contexts::workspaces::application::{
    SessionShellNotice, SessionShellNoticePort, SessionShellWorkspace, SessionShellWorkspacePort,
    ShellClockPort, ShellIdPort, ShellRemoteTarget, WorkspaceShellContextPort,
};
use crate::contexts::workspaces::domain::SessionShellError;
use crate::platform::clock::SystemClock;
use crate::platform::database::NativeDatabase;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};

/// The Tauri event a Shell subscriber listens on.
pub(crate) const SESSION_SHELL_EVENT: &str = "session-shell:notice";

#[derive(Clone)]
pub(crate) struct UuidShellIds;

impl ShellIdPort for UuidShellIds {
    fn next_shell_id(&self) -> String {
        format!("shell-{}", uuid::Uuid::new_v4())
    }

    fn next_attachment_id(&self) -> String {
        format!("attach-{}", uuid::Uuid::new_v4())
    }
}

/// Wall time for what the user reads, monotonic time for what the sweep computes.
///
/// Two clocks rather than one because they answer different questions. A timestamp shown in a tab
/// has to match the rest of the product's timestamps, and an idle window measured against that
/// same clock would reclaim every Shell in the application the moment the host adjusted its time.
pub(crate) struct SystemShellClock {
    started: Instant,
}

impl Default for SystemShellClock {
    fn default() -> Self {
        Self {
            started: Instant::now(),
        }
    }
}

impl ShellClockPort for SystemShellClock {
    fn now(&self) -> String {
        SystemClock.rfc3339()
    }

    fn elapsed_millis(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

/// Where a Shell may be opened, read from the registered session.
///
/// Wraps the existing shell workspace projection rather than adding a second one: two queries
/// answering "where does this session live" would eventually disagree, and the one that disagreed
/// would be the one that opened a shell somewhere the user did not choose. The seat count is the
/// only thing read separately, because the older projection has no reason to carry it.
pub(crate) struct SqliteSessionShellWorkspace {
    database: NativeDatabase,
    context: Arc<dyn WorkspaceShellContextPort>,
}

impl SqliteSessionShellWorkspace {
    pub(crate) fn new(
        database: NativeDatabase,
        context: Arc<dyn WorkspaceShellContextPort>,
    ) -> Self {
        Self { database, context }
    }

    /// Seats are stored as a JSON list on the session. An empty list is the one-seat case, which is
    /// what a session that predates seats looks like.
    fn seat_count(&self, session_id: &str) -> usize {
        let Ok(connection) = self.database.connection() else {
            return 1;
        };
        let stored = connection
            .query_row(
                "SELECT seats FROM sessions WHERE id = ?1",
                params![session_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .ok()
            .flatten()
            .flatten();
        stored
            .and_then(|value| serde_json::from_str::<serde_json::Value>(&value).ok())
            .and_then(|value| value.as_array().map(Vec::len))
            .filter(|count| *count > 0)
            .unwrap_or(1)
    }
}

impl SessionShellWorkspacePort for SqliteSessionShellWorkspace {
    fn resolve(&self, session_id: &str) -> Result<SessionShellWorkspace, SessionShellError> {
        let workspace = self
            .context
            .load_shell_workspace(session_id)
            .map_err(|_| SessionShellError::WorkspaceUnavailable)?;
        // A remote session needs both halves: the endpoint says where, and the binding says which
        // stored connection and revision to open it through. Half of that is not a target — it is a
        // path with nowhere to apply it.
        let remote = match (workspace.remote_endpoint.as_ref(), workspace.ssh_binding) {
            (Some(endpoint), Some(binding)) => Some(ShellRemoteTarget {
                connection_id: binding.connection_id,
                profile_revision: binding.revision,
                path: endpoint.path.clone(),
            }),
            _ => None,
        };
        if remote.is_none() && workspace.root.is_none() {
            return Err(SessionShellError::WorkspaceUnavailable);
        }
        Ok(SessionShellWorkspace {
            root: workspace.root.unwrap_or_default(),
            remote,
            read_only: workspace.read_only,
            seat_count: self.seat_count(session_id),
        })
    }
}

/// One bounded notice per change, over the Tauri event bus.
///
/// The payload is the notice and nothing more. Replay is never carried here: a subscriber that
/// wants scrollback attaches and receives it once, and an event large enough to hold a megabyte of
/// terminal output would be a megabyte crossing the bridge on every burst of output.
#[derive(Clone)]
pub(crate) struct TauriSessionShellNotices {
    app: AppHandle,
}

impl TauriSessionShellNotices {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum TauriShellNotice {
    #[serde(rename_all = "camelCase")]
    Output {
        shell_id: String,
        session_id: String,
        sequence: u64,
        occurred_at: String,
        stream: &'static str,
        data: String,
    },
    #[serde(rename_all = "camelCase")]
    State {
        shell_id: String,
        session_id: String,
        state: &'static str,
        /// Present only for the states that carry one. A reason on a running Shell would be a
        /// field a reader has to learn to ignore.
        reason: Option<String>,
        exit_code: Option<i32>,
        revision: u64,
        occurred_at: String,
    },
}

impl From<SessionShellNotice> for TauriShellNotice {
    fn from(notice: SessionShellNotice) -> Self {
        match notice {
            SessionShellNotice::Output {
                shell_id,
                session_id,
                frame,
            } => Self::Output {
                shell_id: shell_id.as_str().to_string(),
                session_id,
                sequence: frame.sequence,
                occurred_at: frame.occurred_at,
                stream: frame.stream.token(),
                data: frame.data,
            },
            SessionShellNotice::State {
                shell_id,
                session_id,
                state,
                revision,
                occurred_at,
            } => Self::State {
                shell_id: shell_id.as_str().to_string(),
                session_id,
                state: state.token(),
                reason: state.reason().map(str::to_string),
                exit_code: state.exit_code(),
                revision,
                occurred_at,
            },
        }
    }
}

impl SessionShellNoticePort for TauriSessionShellNotices {
    fn publish(&self, notice: SessionShellNotice) {
        let _ = self
            .app
            .emit(SESSION_SHELL_EVENT, TauriShellNotice::from(notice));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::workspaces::domain::{
        shell_reason, SessionShellState, ShellId, ShellOutputFrame, ShellStream,
    };

    #[test]
    fn shell_notices_keep_tagged_camel_case_contracts() {
        let output = TauriShellNotice::from(SessionShellNotice::Output {
            shell_id: ShellId::parse("shell-1").expect("shell id"),
            session_id: "session-1".to_string(),
            frame: ShellOutputFrame {
                sequence: 7,
                occurred_at: "2026-08-22T09:00:00Z".to_string(),
                stream: ShellStream::Pty,
                data: "ready".to_string(),
            },
        });

        assert_eq!(
            serde_json::to_value(output).expect("output"),
            serde_json::json!({
                "type": "output",
                "shellId": "shell-1",
                "sessionId": "session-1",
                "sequence": 7,
                "occurredAt": "2026-08-22T09:00:00Z",
                "stream": "pty",
                "data": "ready"
            })
        );
    }

    /// An exit code and a disconnect reason are different facts, and a notice that flattened both
    /// into one string would make a view parse prose to tell a crash from a dropped transport.
    #[test]
    fn a_state_notice_carries_the_fact_its_state_actually_has() {
        let exited = TauriShellNotice::from(SessionShellNotice::State {
            shell_id: ShellId::parse("shell-1").expect("shell id"),
            session_id: "session-1".to_string(),
            state: SessionShellState::Exited { code: Some(130) },
            revision: 3,
            occurred_at: "2026-08-22T09:00:01Z".to_string(),
        });
        let disconnected = TauriShellNotice::from(SessionShellNotice::State {
            shell_id: ShellId::parse("shell-1").expect("shell id"),
            session_id: "session-1".to_string(),
            state: SessionShellState::Disconnected {
                reason: shell_reason("shell_remote_channel_lost"),
            },
            revision: 4,
            occurred_at: "2026-08-22T09:00:02Z".to_string(),
        });

        let exited = serde_json::to_value(exited).expect("exited");
        let disconnected = serde_json::to_value(disconnected).expect("disconnected");
        assert_eq!(exited["state"], "exited");
        assert_eq!(exited["exitCode"], 130);
        assert_eq!(exited["reason"], serde_json::Value::Null);
        assert_eq!(disconnected["state"], "disconnected");
        assert_eq!(disconnected["reason"], "shell_remote_channel_lost");
        assert_eq!(disconnected["exitCode"], serde_json::Value::Null);
    }
}
