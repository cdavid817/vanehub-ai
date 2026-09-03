//! Noticing that a workspace changed, and telling the console.
//!
//! Two pieces that belong together because one produces what the other carries: a poller that asks
//! each session's provider whether its open directories still look the same, and the adapter that
//! puts the resulting notices on the event channel.
//!
//! The poll goes through the inspection router, which means it works for a remote workspace for the
//! same reason every other read does — the provider is chosen from the session's binding, not from
//! this code. That is the payoff of provider-neutral inspection: "watch the local tree" and "poll
//! the remote host" turn out to be one mechanism, and the honest capability answer for both is
//! `polling` rather than one of them claiming to be live.
//!
//! Nothing here decides *when* to run. The driver owns the cadence and the liveness condition, so a
//! poll that nobody is waiting for is a poll that does not happen.

use crate::contexts::workspaces::application::{
    DirectoryFingerprintState, WorkspaceChangeObserverPort, WorkspaceInspectionRouter,
    WorkspaceInvalidationChange, WorkspaceInvalidationDispatcher, WorkspaceInvalidationNotice,
    WorkspaceInvalidationPublisher, WorkspaceInvalidationScope, WorkspaceInvalidationSource,
    MAX_FINGERPRINT_PATHS,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

pub(crate) const WORKSPACE_INVALIDATION_EVENT: &str = "workspace-invalidation:notice";

/// Compares each poll against the one before it.
///
/// The remembered fingerprints are the whole state. A first look records and announces nothing —
/// the console just read that directory, so it is current by definition, and a notice would send it
/// straight back to refetch what it already has.
pub(crate) struct WorkspaceInvalidationPoller {
    inspection: Arc<WorkspaceInspectionRouter>,
    dispatcher: Arc<WorkspaceInvalidationDispatcher>,
    /// `(session, relative directory)` to the last fingerprint that could be read.
    seen: Mutex<BTreeMap<(String, String), DirectoryFingerprintState>>,
}

impl WorkspaceInvalidationPoller {
    pub(crate) fn new(
        inspection: Arc<WorkspaceInspectionRouter>,
        dispatcher: Arc<WorkspaceInvalidationDispatcher>,
    ) -> Self {
        Self {
            inspection,
            dispatcher,
            seen: Mutex::new(BTreeMap::new()),
        }
    }

    /// One pass over one session's open directories.
    ///
    /// Returns how many changes it found, which is what lets a caller log a poll that is doing
    /// nothing without it having to guess from timing.
    pub(crate) async fn poll_session(&self, session_id: &str, now_ms: u64) -> usize {
        let paths: Vec<String> = self
            .dispatcher
            .observed_directories(session_id, now_ms)
            .into_iter()
            .take(MAX_FINGERPRINT_PATHS)
            .collect();
        if paths.is_empty() {
            return 0;
        }
        let Ok(answers) = self
            .inspection
            .directory_fingerprints(session_id, &paths)
            .await
        else {
            // A workspace that cannot be reached has not been observed to change. Remembered
            // fingerprints are kept rather than cleared, so a change that happened during an
            // outage is still found by the first poll that succeeds afterwards.
            return 0;
        };

        let mut found = 0;
        for answer in answers {
            let key = (session_id.to_string(), answer.relative_path.clone());
            let Ok(mut seen) = self.seen.lock() else {
                return found;
            };
            let previous = seen.get(&key).cloned();
            let scope = compare(&answer.relative_path, previous.as_ref(), &answer.state);
            match &answer.state {
                // Only a state that can be compared next time is worth remembering. Storing
                // `Unreadable` would make the recovery look like a change: the directory would
                // differ from "could not read it", which is not something that happened to it.
                DirectoryFingerprintState::Unreadable => {}
                state => {
                    seen.insert(key, state.clone());
                }
            }
            drop(seen);
            if let Some(scope) = scope {
                found += 1;
                self.dispatcher.observe(
                    session_id,
                    WorkspaceInvalidationSource::Poll,
                    scope,
                    now_ms,
                );
            }
        }
        found
    }

    /// Every session with something open, in one pass.
    pub(crate) async fn poll_observed(&self, now_ms: u64) -> usize {
        let mut found = 0;
        for session_id in self.dispatcher.observed_sessions(now_ms) {
            found += self.poll_session(&session_id, now_ms).await;
        }
        found
    }
}

/// What one directory's before and after mean, if anything.
///
/// Four of the nine combinations are silence, and each for its own reason. Saying so here rather
/// than in the caller keeps the rule in one place, where it can be read against the cases.
fn compare(
    relative_path: &str,
    previous: Option<&DirectoryFingerprintState>,
    current: &DirectoryFingerprintState,
) -> Option<WorkspaceInvalidationScope> {
    match (previous, current) {
        // Nothing to compare against. The console read this directory to get here, so it holds the
        // current contents already.
        (None, _) => None,
        // Losing the ability to look is not an observation that something moved.
        (_, DirectoryFingerprintState::Unreadable) => None,
        (Some(DirectoryFingerprintState::Unreadable), _) => None,
        (
            Some(DirectoryFingerprintState::Known(before)),
            DirectoryFingerprintState::Known(after),
        ) => (before != after).then(|| WorkspaceInvalidationScope::Directory {
            relative_path: relative_path.to_string(),
        }),
        // The directory itself went, so it is its parent's listing that is now wrong. `Path` says
        // that; `Directory` would send the console to refetch a directory that is not there.
        (Some(DirectoryFingerprintState::Known(_)), DirectoryFingerprintState::Missing) => {
            Some(WorkspaceInvalidationScope::Path {
                relative_path: relative_path.to_string(),
                change: WorkspaceInvalidationChange::Removed,
            })
        }
        (Some(DirectoryFingerprintState::Missing), DirectoryFingerprintState::Known(_)) => {
            Some(WorkspaceInvalidationScope::Path {
                relative_path: relative_path.to_string(),
                change: WorkspaceInvalidationChange::Created,
            })
        }
        (Some(DirectoryFingerprintState::Missing), DirectoryFingerprintState::Missing) => None,
    }
}

/// The way a producer elsewhere in the process reports what it saw.
///
/// Exists so the runtime's mutation fanout can hand over a path and a change kind without taking a
/// dependency on the dispatcher, a clock, or the notion of a coalescing window. Everything it needs
/// to know is in the two enums.
pub(crate) struct SystemWorkspaceChangeObserver {
    dispatcher: Arc<WorkspaceInvalidationDispatcher>,
}

impl SystemWorkspaceChangeObserver {
    pub(crate) fn new(dispatcher: Arc<WorkspaceInvalidationDispatcher>) -> Self {
        Self { dispatcher }
    }
}

impl WorkspaceChangeObserverPort for SystemWorkspaceChangeObserver {
    fn observe(
        &self,
        session_id: &str,
        source: WorkspaceInvalidationSource,
        scope: WorkspaceInvalidationScope,
    ) {
        self.dispatcher
            .observe(session_id, source, scope, unix_milliseconds());
    }
}

/// Wall-clock milliseconds, for the coalescing window.
///
/// A clock before the epoch becomes zero rather than an error: the consequence is one odd coalescing
/// cycle on a machine whose clock is badly wrong, which is not worth failing a file write over.
fn unix_milliseconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

/// Puts notices on the event channel.
pub(crate) struct TauriWorkspaceInvalidationNotices {
    app: AppHandle,
}

impl TauriWorkspaceInvalidationNotices {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

/// The wire shape.
///
/// Workspace-relative paths and nothing else. A relative path is a string the console asked for and
/// is already rendering; an absolute one would add the user's home directory, account name, and
/// machine layout to a message whose entire content is "refresh this row".
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TauriInvalidationNotice {
    session_id: String,
    source: &'static str,
    scope: &'static str,
    /// Absent for a workspace-wide notice, which by definition names no path.
    #[serde(skip_serializing_if = "Option::is_none")]
    relative_path: Option<String>,
    /// Present only where a path is. A change kind on a workspace-wide notice would be a field a
    /// reader has to learn to ignore.
    #[serde(skip_serializing_if = "Option::is_none")]
    change: Option<&'static str>,
    sequence: u64,
    occurred_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    coalesced: Option<u32>,
}

impl From<&WorkspaceInvalidationNotice> for TauriInvalidationNotice {
    fn from(notice: &WorkspaceInvalidationNotice) -> Self {
        Self {
            session_id: notice.session_id.clone(),
            source: notice.source.token(),
            scope: notice.scope.token(),
            relative_path: notice.scope.relative_path().map(str::to_string),
            change: match &notice.scope {
                WorkspaceInvalidationScope::Path { change, .. } => Some(change.token()),
                WorkspaceInvalidationScope::Directory { .. }
                | WorkspaceInvalidationScope::Workspace => None,
            },
            sequence: notice.sequence,
            occurred_at: notice.observed_at.clone(),
            coalesced: notice.coalesced,
        }
    }
}

impl WorkspaceInvalidationPublisher for TauriWorkspaceInvalidationNotices {
    fn publish(&self, notice: &WorkspaceInvalidationNotice) {
        // A dropped emit is dropped. The alternative is propagating a delivery failure back into
        // the poll or the file write that produced it, and neither of those can do anything about
        // a window that is closing.
        let _ = self.app.emit(
            WORKSPACE_INVALIDATION_EVENT,
            TauriInvalidationNotice::from(notice),
        );
    }
}
