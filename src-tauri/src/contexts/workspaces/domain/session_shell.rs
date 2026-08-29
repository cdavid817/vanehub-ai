//! Identities, state, and typed failures for a retained Session Shell.
//!
//! A Session Shell outlives the view that opened it. That is the whole point of the type, and it is
//! why the identities below are separate: the Shell is the process, the attachment is one view's
//! claim on it, and the create request is the client's idempotency key. Collapsing any two of them
//! is what lets a tab switch kill a build, a late cleanup steal a live view, or a retried click
//! spawn a second shell.

/// A bounded opaque identifier.
///
/// Every one of these crosses a command boundary, so the bound is the point: a client that could
/// send an unbounded id could make the registry's keys unbounded too.
macro_rules! bounded_shell_id {
    ($name:ident, $field:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub(crate) struct $name(String);

        impl $name {
            pub(crate) fn parse(value: impl Into<String>) -> Result<Self, SessionShellError> {
                let value = value.into();
                if value.is_empty()
                    || value.chars().count() > MAX_SHELL_IDENTIFIER_LENGTH
                    || value.chars().any(char::is_control)
                {
                    return Err(SessionShellError::InvalidIdentifier { field: $field });
                }
                Ok(Self(value))
            }

            pub(crate) fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

pub(crate) const MAX_SHELL_IDENTIFIER_LENGTH: usize = 128;
pub(crate) const MAX_SHELL_TITLE_LENGTH: usize = 64;

bounded_shell_id!(ShellId, "shell id");
bounded_shell_id!(ShellAttachmentId, "attachment id");
bounded_shell_id!(ShellCreateRequestId, "create request id");
bounded_shell_id!(ShellReasonCode, "reason code");

impl ShellReasonCode {
    /// Builds a reason code from anything, without a fallible parse.
    ///
    /// Reason codes are produced on failure paths — a lost transport, an evicted buffer, a runtime
    /// that would not start. A constructor that could itself fail there would leave each call site
    /// choosing between a panic and a lost reason, and the reason is the only thing the user has to
    /// go on. So the invariant is enforced by repair rather than rejection: control characters are
    /// dropped, the length is bounded, and an input with nothing left says so.
    pub(crate) fn sanitized(code: &str) -> Self {
        let cleaned = code
            .chars()
            .filter(|character| !character.is_control())
            .take(MAX_SHELL_IDENTIFIER_LENGTH)
            .collect::<String>();
        if cleaned.is_empty() {
            return Self("shell_reason_unavailable".to_string());
        }
        Self(cleaned)
    }
}

/// What the user named a Shell.
///
/// Bounded and control-free because it is rendered in a tab strip and written into diagnostics; a
/// title carrying a newline or an escape sequence would reshape both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellTitle(String);

impl ShellTitle {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, SessionShellError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty()
            || trimmed.chars().count() > MAX_SHELL_TITLE_LENGTH
            || trimmed.chars().any(char::is_control)
        {
            return Err(SessionShellError::InvalidTitle);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Where a Shell is in its life.
///
/// `Exited` and `Closed` are different endings and are kept apart: a process that ended on its own
/// is still worth reading, and the registry keeps it attachable for replay, whereas a Shell the
/// user closed is gone. Reporting either as the other would either hide a crash or resurrect a
/// tab the user dismissed.
///
/// The three intermediate states are the other half of that distinction, on the way in and on the
/// way out. `Opening` exists so a Shell is addressable before its workers can publish anything;
/// `Closing`, `Reaping`, and `CloseFailed` exist because "close was asked for" and "the process is
/// gone" are different facts, and a state model with only the first is a model in which the UI
/// reports a terminated shell while the process is still running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionShellState {
    Starting,
    /// Registered, but the runtime has not committed ownership yet. Not writable: a keystroke
    /// delivered here would race the very handoff that decides whether the Shell exists at all.
    Opening,
    Running,
    /// Close was requested and one bounded attempt is in progress. Still addressable, still holding
    /// its capacity, and explicitly not terminal.
    Closing,
    /// A close attempt reached its deadline and ownership moved to the retained Reaper. The
    /// resources still exist, so the Shell still consumes its capacity.
    Reaping,
    /// Cleanup failed with a reported reason and the handles are still owned here. `retryable` is
    /// what the UI needs to decide whether offering a retry is honest.
    CloseFailed {
        reason: ShellReasonCode,
        retryable: bool,
    },
    /// The process ended by itself. `None` when the runtime could not report a code — never `0`,
    /// which would report an unknown ending as a clean one.
    Exited {
        code: Option<i32>,
    },
    /// A remote channel lost its transport. Recoverable only if the descriptor says so.
    Disconnected {
        reason: ShellReasonCode,
    },
    Failed {
        reason: ShellReasonCode,
    },
    /// Closed on request, on idle, or at shutdown, *and confirmed terminal*.
    Closed,
}

impl SessionShellState {
    pub(crate) fn token(&self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Opening => "opening",
            Self::Running => "running",
            Self::Closing => "closing",
            Self::Reaping => "reaping",
            Self::CloseFailed { .. } => "close_failed",
            Self::Exited { .. } => "exited",
            Self::Disconnected { .. } => "disconnected",
            Self::Failed { .. } => "failed",
            Self::Closed => "closed",
        }
    }

    /// The reason code the state carries, for the states that have one.
    ///
    /// Kept apart from the exit code rather than flattened into one string, because a reader that
    /// had to parse prose could not tell a crash from a dropped transport.
    pub(crate) fn reason(&self) -> Option<&str> {
        match self {
            Self::Disconnected { reason }
            | Self::Failed { reason }
            | Self::CloseFailed { reason, .. } => Some(reason.as_str()),
            _ => None,
        }
    }

    /// Whether cleanup has been asked for and is not finished.
    ///
    /// The predicate a caller needs before claiming a session was deleted or a sweep closed a
    /// Shell: these three states each mean an operating-system process or an SSH channel is still
    /// out there under this application's ownership.
    pub(crate) fn is_cleanup_pending(&self) -> bool {
        matches!(
            self,
            Self::Closing | Self::Reaping | Self::CloseFailed { .. }
        )
    }

    /// Whether the runtime has committed ownership and the Shell is usable.
    pub(crate) fn is_opening(&self) -> bool {
        matches!(self, Self::Starting | Self::Opening)
    }

    /// Whether moving from here to `next` is a move the aggregate allows.
    ///
    /// One table rather than a check at each writer, because the illegal moves are the ones nobody
    /// writes deliberately: a startup completing after the shell already exited and overwriting the
    /// exit with `Running`, a monitor thread turning a requested close into a natural exit, or a
    /// stale reaper reopening a Shell that has already been finalized. Each of those is a plausible
    /// line of code at its own call site and a lie about the process at this one.
    pub(crate) fn may_transition_to(&self, next: &Self) -> bool {
        if self == next || matches!(self, Self::Closed) {
            return false;
        }
        match (self, next) {
            // An ended Shell still has an entry, a replay buffer, and — until a runtime confirms
            // otherwise — workers and handles. Closing it is therefore a real operation, and the
            // only one it admits: it cannot come back to life, and it cannot end a second time.
            (
                Self::Exited { .. } | Self::Failed { .. },
                Self::Closing | Self::Reaping | Self::CloseFailed { .. } | Self::Closed,
            ) => true,
            (Self::Exited { .. } | Self::Failed { .. }, _) => false,
            // Startup can complete, end early, or be closed before it completes. It cannot be
            // finalized as `Closed` directly: a startup that never committed has nothing to
            // confirm, and a rollback reports `Failed` with the reason it rolled back for.
            (
                Self::Starting | Self::Opening,
                Self::Running
                | Self::Closing
                | Self::Reaping
                | Self::Exited { .. }
                | Self::Disconnected { .. }
                | Self::Failed { .. },
            ) => true,
            (
                Self::Running,
                Self::Closing
                | Self::Exited { .. }
                | Self::Disconnected { .. }
                | Self::Failed { .. },
            ) => true,
            // A dropped transport is not an ending, so a disconnected Shell can still be closed,
            // and can still be observed to have ended underneath.
            (
                Self::Disconnected { .. },
                Self::Closing | Self::Exited { .. } | Self::Failed { .. },
            ) => true,
            // Once close is under way the close operation owns the ending. A monitor observing the
            // child exit here must not report `Exited`: the user asked for a close, and reporting
            // the ending it produced as a spontaneous exit would lose the request.
            (Self::Closing, Self::Reaping | Self::CloseFailed { .. } | Self::Closed) => true,
            (Self::Reaping, Self::CloseFailed { .. } | Self::Closed) => true,
            // A failed close is retryable in place: the next attempt re-enters `Closing`, and a
            // reaper that succeeds late can finalize it directly.
            (Self::CloseFailed { .. }, Self::Closing | Self::Reaping | Self::Closed) => true,
            (Self::CloseFailed { .. }, Self::CloseFailed { .. }) => true,
            _ => false,
        }
    }

    /// The process exit code, when the runtime reported one.
    pub(crate) fn exit_code(&self) -> Option<i32> {
        match self {
            Self::Exited { code } => *code,
            _ => None,
        }
    }

    /// Whether the runtime can still accept input. A view uses this to decide whether to bind a
    /// keyboard, and it is deliberately narrower than "the Shell still exists".
    ///
    /// `Opening` is excluded even though the Shell is addressable there. A write accepted before
    /// the runtime committed ownership would have nowhere to go, and answering it with success
    /// would tell the caller a keystroke was delivered to a process that may not exist yet.
    pub(crate) fn accepts_input(&self) -> bool {
        matches!(self, Self::Starting | Self::Running)
    }

    /// Whether the registry should still hold it. An exited Shell stays until it is closed or the
    /// idle sweep reclaims it, because its output is what the user came back to read.
    pub(crate) fn is_terminal(&self) -> bool {
        matches!(self, Self::Closed)
    }

    /// Whether the runtime behind it has stopped.
    ///
    /// A different question from `is_terminal`, and the difference is the point: an exited Shell
    /// has ended and is still held. Once a Shell has ended it does not end again — a second
    /// terminal state would publish two endings for one Shell, and a reader counting them would
    /// see work that never happened.
    pub(crate) fn has_ended(&self) -> bool {
        matches!(
            self,
            Self::Exited { .. } | Self::Failed { .. } | Self::Closed
        )
    }
}

/// Which stream a frame came from.
///
/// `Pty` is not a synonym for stdout. A PTY hands back one interleaved stream, and labelling it
/// `stdout` would let a reader believe the runtime separated two things it never saw apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellStream {
    Pty,
    Stdout,
    Stderr,
    System,
}

impl ShellStream {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Pty => "pty",
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
            Self::System => "system",
        }
    }
}

/// One retained chunk of Shell output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellOutputFrame {
    pub(crate) sequence: u64,
    pub(crate) occurred_at: String,
    pub(crate) stream: ShellStream,
    pub(crate) data: String,
}

/// Output the registry no longer holds.
///
/// One contiguous range, because eviction only ever removes from the oldest end. A reader that saw
/// two gaps could not tell which of them it was looking at the far side of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellReplayGap {
    pub(crate) from_sequence: u64,
    pub(crate) to_sequence: u64,
    pub(crate) reason: ShellReasonCode,
}

/// Whether something is running in the foreground of a Shell.
///
/// Three states, not two. `Unknown` is what an opaque runtime honestly reports, and collapsing it
/// into `Absent` would let a close confirmation say "nothing is running" about a shell that is
/// midway through a deploy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ShellForegroundProcessState {
    Present,
    Absent,
    /// The default, because a runtime that has not been asked has not answered. Starting from
    /// absent would let a close confirmation say "nothing is running" before anyone looked.
    #[default]
    Unknown,
}

impl ShellForegroundProcessState {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Absent => "absent",
            Self::Unknown => "unknown",
        }
    }
}

/// Which ceiling a create request hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellCapacityScope {
    Session,
    Application,
}

/// Every way a Shell operation can fail, as a value.
///
/// Typed rather than a string because the frontend has to tell these apart: a stale attachment is a
/// race the view recovers from by reattaching, a capacity failure is something the user must act
/// on, and a disconnect is a state the UI keeps showing alongside whatever replay it still holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionShellError {
    InvalidIdentifier {
        field: &'static str,
    },
    InvalidTitle,
    NotFound,
    /// The operation named an attachment that is no longer the Shell's current one.
    AttachmentStale,
    CapacityReached {
        scope: ShellCapacityScope,
    },
    /// A multi-seat session asked for a Shell without saying whose it is.
    SeatRequired,
    /// The session's workspace forbids opening a Shell at all.
    PolicyDenied,
    WorkspaceUnavailable,
    /// The Shell exists but cannot take this operation in its current state.
    NotAcceptingInput {
        state: &'static str,
    },
    RuntimeUnavailable {
        reason: ShellReasonCode,
    },
    Runtime {
        reason: ShellReasonCode,
    },
}

impl SessionShellError {
    /// The stable token a command returns. Part of the contract, so it is written once here rather
    /// than formatted at each boundary.
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::InvalidIdentifier { .. } => "shell_invalid_identifier",
            Self::InvalidTitle => "shell_invalid_title",
            Self::NotFound => "shell_not_found",
            Self::AttachmentStale => "shell_attachment_stale",
            Self::CapacityReached {
                scope: ShellCapacityScope::Session,
            } => "shell_session_capacity_reached",
            Self::CapacityReached {
                scope: ShellCapacityScope::Application,
            } => "shell_application_capacity_reached",
            Self::SeatRequired => "shell_seat_required",
            Self::PolicyDenied => "shell_policy_denied",
            Self::WorkspaceUnavailable => "shell_workspace_unavailable",
            Self::NotAcceptingInput { .. } => "shell_not_accepting_input",
            Self::RuntimeUnavailable { .. } => "shell_runtime_unavailable",
            Self::Runtime { .. } => "shell_runtime_error",
        }
    }
}

impl std::fmt::Display for SessionShellError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::workspaces::domain::session_shell_lifecycle::shell_reason_code;

    fn reason(code: &str) -> ShellReasonCode {
        ShellReasonCode::sanitized(code)
    }

    /// The defect this table exists for. A Shell that echoes and exits does so before the startup
    /// path gets to write `Running`, and an unconditional write there would replace a real ending
    /// with a claim that the shell is live — after which nothing ever ends it.
    #[test]
    fn a_startup_completion_cannot_overwrite_an_ending_that_already_happened() {
        let exited = SessionShellState::Exited { code: Some(0) };

        assert!(SessionShellState::Opening.may_transition_to(&SessionShellState::Running));
        assert!(SessionShellState::Opening.may_transition_to(&exited));
        assert!(!exited.may_transition_to(&SessionShellState::Running));
        assert!(!SessionShellState::Closed.may_transition_to(&SessionShellState::Running));
        assert!(!SessionShellState::Failed {
            reason: reason("x")
        }
        .may_transition_to(&SessionShellState::Running));
    }

    /// A confirmed ending does not happen twice. Two terminal states for one Shell would publish
    /// two endings, and anything counting them would see work that never happened.
    #[test]
    fn an_ended_shell_admits_only_being_closed() {
        for ended in [
            SessionShellState::Exited { code: None },
            SessionShellState::Failed {
                reason: reason("x"),
            },
        ] {
            assert!(ended.has_ended(), "{}", ended.token());
            // Still a real operation: the entry, the replay buffer, and — until a runtime says
            // otherwise — the workers are all there, and dismissing the tab has to reclaim them.
            assert!(ended.may_transition_to(&SessionShellState::Closing));
            assert!(ended.may_transition_to(&SessionShellState::Closed));
            for next in [
                SessionShellState::Running,
                SessionShellState::Opening,
                SessionShellState::Exited { code: Some(1) },
                SessionShellState::Disconnected {
                    reason: reason("x"),
                },
            ] {
                assert!(
                    !ended.may_transition_to(&next),
                    "{} -> {}",
                    ended.token(),
                    next.token()
                );
            }
        }
    }

    /// `Closed` is the one state nothing leaves. It is the only one meaning every owned resource was
    /// confirmed gone, so anything after it would be a claim about a Shell with nothing left to
    /// claim.
    #[test]
    fn a_finalized_shell_admits_no_transition_at_all() {
        for next in [
            SessionShellState::Running,
            SessionShellState::Closing,
            SessionShellState::Reaping,
            SessionShellState::Exited { code: Some(0) },
            SessionShellState::CloseFailed {
                reason: reason("x"),
                retryable: true,
            },
        ] {
            assert!(
                !SessionShellState::Closed.may_transition_to(&next),
                "closed -> {}",
                next.token()
            );
        }
    }

    /// Once close is under way the close operation owns the ending. The monitor thread observing
    /// the child die is the *expected* consequence of the kill, and reporting it as a spontaneous
    /// exit would lose the fact that a user asked for this.
    #[test]
    fn a_close_in_progress_is_not_reclassified_as_a_natural_exit() {
        assert!(!SessionShellState::Closing
            .may_transition_to(&SessionShellState::Exited { code: Some(0) }));
        assert!(!SessionShellState::Reaping
            .may_transition_to(&SessionShellState::Exited { code: Some(0) }));
        assert!(SessionShellState::Closing.may_transition_to(&SessionShellState::Closed));
        assert!(SessionShellState::Closing.may_transition_to(&SessionShellState::Reaping));
        assert!(SessionShellState::Reaping.may_transition_to(&SessionShellState::Closed));
    }

    /// A failed close keeps its handles, so the next attempt re-enters the same close rather than
    /// starting a competing one, and a reaper that succeeds late can still finalize it.
    #[test]
    fn a_failed_close_is_retryable_in_place() {
        let failed = SessionShellState::CloseFailed {
            reason: reason(shell_reason_code::CLOSE_DEADLINE_REACHED),
            retryable: true,
        };

        assert!(failed.is_cleanup_pending());
        assert!(!failed.has_ended());
        assert!(!failed.is_terminal());
        assert!(failed.may_transition_to(&SessionShellState::Closing));
        assert!(failed.may_transition_to(&SessionShellState::Closed));
        assert!(!failed.may_transition_to(&SessionShellState::Running));
    }

    /// Cleanup pending is the predicate a caller needs before claiming a session was deleted: each
    /// of these three means a process or a channel is still out there under this application's
    /// ownership.
    #[test]
    fn cleanup_pending_covers_exactly_the_unconfirmed_states() {
        assert!(SessionShellState::Closing.is_cleanup_pending());
        assert!(SessionShellState::Reaping.is_cleanup_pending());
        assert!(!SessionShellState::Running.is_cleanup_pending());
        assert!(!SessionShellState::Opening.is_cleanup_pending());
        assert!(!SessionShellState::Closed.is_cleanup_pending());
        assert!(!SessionShellState::Exited { code: Some(0) }.is_cleanup_pending());
    }

    /// A keystroke accepted before the runtime committed ownership would have nowhere to go, and
    /// answering it with success would report a delivery that did not happen.
    #[test]
    fn an_opening_shell_is_addressable_but_not_writable() {
        assert!(!SessionShellState::Opening.accepts_input());
        assert!(!SessionShellState::Closing.accepts_input());
        assert!(SessionShellState::Running.accepts_input());
        assert!(SessionShellState::Opening.is_opening());
    }

    /// These tokens cross the command boundary and the frontend switches on them, so they are part
    /// of the contract rather than a debug rendering.
    #[test]
    fn every_state_has_a_distinct_stable_token() {
        let tokens = [
            SessionShellState::Starting,
            SessionShellState::Opening,
            SessionShellState::Running,
            SessionShellState::Closing,
            SessionShellState::Reaping,
            SessionShellState::CloseFailed {
                reason: reason("x"),
                retryable: true,
            },
            SessionShellState::Exited { code: None },
            SessionShellState::Disconnected {
                reason: reason("x"),
            },
            SessionShellState::Failed {
                reason: reason("x"),
            },
            SessionShellState::Closed,
        ]
        .map(|state| state.token());
        let mut unique = tokens.to_vec();
        unique.sort_unstable();
        unique.dedup();

        assert_eq!(unique.len(), tokens.len());
        assert!(tokens.contains(&"close_failed"));
    }

    /// A close failure carries its reason the same way a disconnect does, so a reader never has to
    /// parse prose to tell "the child would not die" from "the transport dropped".
    #[test]
    fn a_close_failure_reports_its_reason_and_never_an_exit_code() {
        let failed = SessionShellState::CloseFailed {
            reason: reason(shell_reason_code::TERMINATE_FAILED),
            retryable: false,
        };

        assert_eq!(failed.reason(), Some(shell_reason_code::TERMINATE_FAILED));
        assert_eq!(failed.exit_code(), None);
    }
}
