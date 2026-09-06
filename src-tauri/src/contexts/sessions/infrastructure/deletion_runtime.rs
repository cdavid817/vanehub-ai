//! Quiescence: stopping everything this application runs for a session and waiting for proof.
//!
//! The pre-existing `stop_session_activity` asks for things to stop. This waits for them to
//! have stopped, within a bound, and reports what is still alive when the bound runs out. A
//! cancellation that was accepted is not a writer that has exited.

use super::runtime_support::AgentSessionRuntimeAdapter;
use crate::contexts::sessions::application::{
    QuiescenceReport, SessionDeletionRuntimePort, SessionsApplicationError,
};
use crate::contexts::workspaces::api::WorkspaceError;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(100);

impl SessionDeletionRuntimePort for AgentSessionRuntimeAdapter {
    fn quiesce(
        &self,
        session_id: &str,
        deadline: Duration,
    ) -> Result<QuiescenceReport, SessionsApplicationError> {
        let started = Instant::now();
        let remaining = || deadline.saturating_sub(started.elapsed());
        let mut blockers = Vec::new();
        let runtime = self.published_agent_runtime()?;

        // Generation: request the stop, then wait for the correlation to clear.
        match runtime.stop_generation(session_id) {
            Ok(_) => {}
            Err(
                crate::contexts::agent_runtime::api::AgentRuntimeApplicationError::SessionNotFound(
                    _,
                ),
            ) => {}
            Err(error) => {
                return Err(SessionsApplicationError::Runtime(error.to_string()));
            }
        }
        loop {
            let live = runtime
                .active_generation_correlation(session_id)
                .map_err(|error| SessionsApplicationError::Runtime(error.to_string()))?;
            if live.is_none() {
                break;
            }
            if remaining().is_zero() {
                blockers.push("generation".to_string());
                break;
            }
            std::thread::sleep(POLL_INTERVAL);
        }
        if runtime
            .has_live_tool_approval_waiter(session_id)
            .unwrap_or(true)
        {
            blockers.push("tool_approval_waiter".to_string());
        }

        // Background commands: kill and wait for every supervisor to settle.
        if !runtime.reap_background_commands_and_wait(session_id, remaining()) {
            blockers.push("background_command".to_string());
        }

        // Shells: close is strict already; retry it while a close is still being confirmed.
        loop {
            match self.workspaces().kill_shells_for_session(session_id) {
                Ok(()) => break,
                Err(WorkspaceError::Conflict(_)) if !remaining().is_zero() => {
                    std::thread::sleep(POLL_INTERVAL);
                }
                Err(WorkspaceError::Conflict(_)) => {
                    blockers.push("shell".to_string());
                    break;
                }
                Err(error) => return Err(SessionsApplicationError::Workspace(error.to_string())),
            }
        }
        if self.workspaces().live_session_shell_count(session_id) > 0
            && !blockers.iter().any(|blocker| blocker == "shell")
        {
            blockers.push("shell".to_string());
        }

        Ok(QuiescenceReport {
            quiet: blockers.is_empty(),
            blockers,
        })
    }
}
