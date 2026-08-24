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
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionShellState {
    Starting,
    Running,
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
    /// Closed on request, on idle, or at shutdown.
    Closed,
}

impl SessionShellState {
    pub(crate) fn token(&self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
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
            Self::Disconnected { reason } | Self::Failed { reason } => Some(reason.as_str()),
            _ => None,
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
