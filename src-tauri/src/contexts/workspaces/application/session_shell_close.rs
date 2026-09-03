//! What ending a Shell answers with.
//!
//! Ending a Shell is not a boolean. A `Result<(), _>` forces every caller to collapse four
//! genuinely different situations — the process is gone, the process is still there and somebody is
//! still working on it, cleanup failed and can be retried, and there was nothing to end — into
//! either "success" or "error". Callers then choose success, because three of the four are not
//! errors from where they stand, and the application ends up reporting a session as deleted while
//! one of its shells is still running.
//!
//! So the outcome is a value with a name, and the aggregate over several Shells keeps every one of
//! those names rather than a count of failures.

use crate::contexts::workspaces::domain::{
    SessionShellState, ShellGeneration, ShellId, ShellReasonCode,
};

/// What ending one Shell actually achieved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellCloseDisposition {
    /// The owned process or channel is confirmed gone and the Shell has been finalized.
    ClosedConfirmed,
    /// The bounded attempt ran out and the retained Reaper now owns the continuation. The Shell,
    /// its handles, and its capacity are all still held.
    Reaping,
    /// Cleanup failed with a reported reason and ownership stayed where it was.
    CloseFailed,
    /// There was nothing left to end: the Shell had already reached a confirmed terminal state.
    AlreadyTerminal,
}

impl ShellCloseDisposition {
    /// The stable token a command returns and the frontend switches on.
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::ClosedConfirmed => "closed",
            Self::Reaping => "reaping",
            Self::CloseFailed => "close_failed",
            Self::AlreadyTerminal => "already_terminal",
        }
    }

    /// Whether the Shell can be considered finished with. Deliberately not "did the call return
    /// without an error": `Reaping` and `CloseFailed` both return without an error and both mean a
    /// process may still be alive.
    pub(crate) fn is_settled(self) -> bool {
        matches!(self, Self::ClosedConfirmed | Self::AlreadyTerminal)
    }
}

/// One Shell's close outcome, in full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionShellCloseResult {
    pub(crate) shell_id: ShellId,
    /// Which attempt at this Shell the result belongs to, so a retry can be matched to the
    /// operation it is retrying rather than to whatever now answers to the same id.
    pub(crate) generation: ShellGeneration,
    pub(crate) disposition: ShellCloseDisposition,
    /// Present only for a settled disposition. An unsettled close has no final state *because*
    /// nothing final has been observed, and inventing one here is the whole defect.
    pub(crate) final_state: Option<SessionShellState>,
    pub(crate) reason: Option<ShellReasonCode>,
    pub(crate) retryable: bool,
    /// Which bounded attempt this was, counting from one.
    pub(crate) attempt: u32,
    pub(crate) cleanup_deadline_reached: bool,
}

impl SessionShellCloseResult {
    pub(crate) fn confirmed(
        shell_id: ShellId,
        generation: ShellGeneration,
        final_state: SessionShellState,
        attempt: u32,
    ) -> Self {
        Self {
            shell_id,
            generation,
            disposition: ShellCloseDisposition::ClosedConfirmed,
            final_state: Some(final_state),
            reason: None,
            retryable: false,
            attempt,
            cleanup_deadline_reached: false,
        }
    }

    pub(crate) fn already_terminal(
        shell_id: ShellId,
        generation: ShellGeneration,
        final_state: SessionShellState,
    ) -> Self {
        Self {
            shell_id,
            generation,
            disposition: ShellCloseDisposition::AlreadyTerminal,
            final_state: Some(final_state),
            reason: None,
            retryable: false,
            attempt: 0,
            cleanup_deadline_reached: false,
        }
    }

    pub(crate) fn reaping(
        shell_id: ShellId,
        generation: ShellGeneration,
        reason: ShellReasonCode,
        attempt: u32,
    ) -> Self {
        Self {
            shell_id,
            generation,
            disposition: ShellCloseDisposition::Reaping,
            final_state: None,
            reason: Some(reason),
            retryable: true,
            attempt,
            cleanup_deadline_reached: true,
        }
    }

    pub(crate) fn failed(
        shell_id: ShellId,
        generation: ShellGeneration,
        reason: ShellReasonCode,
        retryable: bool,
        attempt: u32,
        cleanup_deadline_reached: bool,
    ) -> Self {
        Self {
            shell_id,
            generation,
            disposition: ShellCloseDisposition::CloseFailed,
            final_state: None,
            reason: Some(reason),
            retryable,
            attempt,
            cleanup_deadline_reached,
        }
    }
}

/// Every Shell a session-wide cleanup touched, and what happened to each.
///
/// The entries are kept rather than reduced to counts, because the identities are what a retry
/// needs. A report saying "one of eleven failed" without saying which one leaves the caller to
/// close all eleven again, and closing a Shell that succeeded the first time is how a retry becomes
/// destructive.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SessionShellCleanupReport {
    entries: Vec<SessionShellCloseResult>,
}

impl SessionShellCleanupReport {
    pub(crate) fn push(&mut self, result: SessionShellCloseResult) {
        self.entries.push(result);
    }

    pub(crate) fn entries(&self) -> &[SessionShellCloseResult] {
        &self.entries
    }

    pub(crate) fn requested(&self) -> usize {
        self.entries.len()
    }

    fn count(&self, disposition: ShellCloseDisposition) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.disposition == disposition)
            .count()
    }

    pub(crate) fn closed_confirmed(&self) -> usize {
        self.count(ShellCloseDisposition::ClosedConfirmed)
    }

    pub(crate) fn already_terminal(&self) -> usize {
        self.count(ShellCloseDisposition::AlreadyTerminal)
    }

    pub(crate) fn reaping(&self) -> usize {
        self.count(ShellCloseDisposition::Reaping)
    }

    pub(crate) fn failed(&self) -> usize {
        self.count(ShellCloseDisposition::CloseFailed)
    }

    /// Whether every Shell reached a confirmed terminal state.
    ///
    /// This is the predicate an archive or a delete has to consult before finalizing. It is not
    /// "no errors were returned": `Reaping` is not an error and still means a process is alive.
    pub(crate) fn is_complete(&self) -> bool {
        self.entries
            .iter()
            .all(|entry| entry.disposition.is_settled())
    }

    /// The Shells still holding resources, for a retry that touches only what is left.
    pub(crate) fn unconfirmed(&self) -> Vec<&SessionShellCloseResult> {
        self.entries
            .iter()
            .filter(|entry| !entry.disposition.is_settled())
            .collect()
    }
}

/// What a runtime adapter can honestly say after one bounded close attempt.
///
/// There is no error variant, and that is the point. Every way a close can go wrong here leaves
/// resources owned by the adapter, so "failed" and "still mine" are the same fact; a `Result` would
/// let a caller write `let _ =` and drop the only reference to a live child.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ShellRuntimeCloseOutcome {
    /// The process or channel is gone and every worker reported itself complete.
    Confirmed,
    /// This runtime does not hold that `(shell_id, generation)`. Either it never did, or an earlier
    /// attempt already finalized it.
    NotHeld,
    /// The attempt ended without confirmation. The adapter still owns every handle, the Shell is
    /// still addressable at the same generation, and another attempt will continue this one.
    Retained {
        reason: ShellReasonCode,
        retryable: bool,
    },
}

impl ShellRuntimeCloseOutcome {
    /// Whether the adapter is done with the Shell. `NotHeld` counts: there is nothing left here to
    /// own, whoever owned it last.
    pub(crate) fn is_released(&self) -> bool {
        matches!(self, Self::Confirmed | Self::NotHeld)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::workspaces::domain::{shell_reason, shell_reason_code};

    fn shell(id: &str) -> ShellId {
        ShellId::parse(id).expect("shell id")
    }

    /// The distinction the whole module exists for. Three of these four return without an error and
    /// only two of them mean the process is gone.
    #[test]
    fn only_a_confirmed_or_already_terminal_close_counts_as_settled() {
        assert!(ShellCloseDisposition::ClosedConfirmed.is_settled());
        assert!(ShellCloseDisposition::AlreadyTerminal.is_settled());
        assert!(!ShellCloseDisposition::Reaping.is_settled());
        assert!(!ShellCloseDisposition::CloseFailed.is_settled());
    }

    #[test]
    fn an_unsettled_result_reports_no_final_state() {
        let reaping = SessionShellCloseResult::reaping(
            shell("shell-1"),
            ShellGeneration::new(1),
            shell_reason(shell_reason_code::CLOSE_DEADLINE_REACHED),
            1,
        );

        // A final state here would be an assertion about a process nobody observed ending.
        assert_eq!(reaping.final_state, None);
        assert!(reaping.retryable);
        assert!(reaping.cleanup_deadline_reached);
    }

    /// A report saying "one of these failed" without saying which one forces a caller to retry all
    /// of them, and retrying a Shell that already closed is how a retry becomes destructive.
    #[test]
    fn a_mixed_report_keeps_every_identity_and_refuses_to_call_itself_complete() {
        let mut report = SessionShellCleanupReport::default();
        report.push(SessionShellCloseResult::confirmed(
            shell("shell-1"),
            ShellGeneration::new(1),
            SessionShellState::Closed,
            1,
        ));
        report.push(SessionShellCloseResult::already_terminal(
            shell("shell-2"),
            ShellGeneration::new(1),
            SessionShellState::Exited { code: Some(0) },
        ));
        report.push(SessionShellCloseResult::reaping(
            shell("shell-3"),
            ShellGeneration::new(2),
            shell_reason(shell_reason_code::CLOSE_DEADLINE_REACHED),
            1,
        ));
        report.push(SessionShellCloseResult::failed(
            shell("shell-4"),
            ShellGeneration::new(1),
            shell_reason(shell_reason_code::REAPER_CAPACITY_EXHAUSTED),
            true,
            1,
            false,
        ));

        assert_eq!(report.requested(), 4);
        assert_eq!(report.closed_confirmed(), 1);
        assert_eq!(report.already_terminal(), 1);
        assert_eq!(report.reaping(), 1);
        assert_eq!(report.failed(), 1);
        assert!(!report.is_complete());
        assert_eq!(
            report
                .unconfirmed()
                .iter()
                .map(|entry| entry.shell_id.as_str())
                .collect::<Vec<_>>(),
            vec!["shell-3", "shell-4"]
        );
    }

    #[test]
    fn a_report_of_settled_outcomes_is_complete() {
        let mut report = SessionShellCleanupReport::default();
        report.push(SessionShellCloseResult::confirmed(
            shell("shell-1"),
            ShellGeneration::new(1),
            SessionShellState::Closed,
            1,
        ));
        report.push(SessionShellCloseResult::already_terminal(
            shell("shell-2"),
            ShellGeneration::new(1),
            SessionShellState::Exited { code: Some(0) },
        ));

        assert!(report.is_complete());
        assert!(report.unconfirmed().is_empty());
    }

    /// `NotHeld` releases the adapter even though nothing was killed: there is nothing left here to
    /// own. `Retained` does not, whatever the reason says.
    #[test]
    fn a_retained_outcome_never_reads_as_released() {
        assert!(ShellRuntimeCloseOutcome::Confirmed.is_released());
        assert!(ShellRuntimeCloseOutcome::NotHeld.is_released());
        assert!(!ShellRuntimeCloseOutcome::Retained {
            reason: shell_reason(shell_reason_code::TERMINATE_FAILED),
            retryable: false,
        }
        .is_released());
    }
}
