use super::error::EvidenceDomainError;
use super::identity::{BoundedLabel, SafeFingerprint, SafeReasonCode};
use super::safety::{RedactedCommandDisplay, RelativeDisplayPath, SafeBasename};

/// The schema this build writes. Persisted with every event so a later build can tell what shape a
/// row was written in without guessing from its contents.
pub(crate) const EVIDENCE_SCHEMA_VERSION: u16 = 1;

/// Serialized bound for one payload. Generous for metadata, far below anything that could hold a
/// transcript or a diff, and checked after serialization so no combination of bounded fields can
/// add up to something unbounded.
pub(crate) const MAX_SAFE_PAYLOAD_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum EvidenceKind {
    RunStarted,
    RunCompleted,
    AgentDelegated,
    AgentCompleted,
    ToolStarted,
    ToolCompleted,
    CommandStarted,
    CommandCompleted,
    ShellOpened,
    ShellClosed,
    FileMutationObserved,
    ReviewDecisionRecorded,
    VerificationCompleted,
    UsageObserved,
    OperationFailed,
    CoverageGapRecorded,
}

impl EvidenceKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::RunStarted => "run.started",
            Self::RunCompleted => "run.completed",
            Self::AgentDelegated => "agent.delegated",
            Self::AgentCompleted => "agent.completed",
            Self::ToolStarted => "tool.started",
            Self::ToolCompleted => "tool.completed",
            Self::CommandStarted => "command.started",
            Self::CommandCompleted => "command.completed",
            Self::ShellOpened => "shell.opened",
            Self::ShellClosed => "shell.closed",
            Self::FileMutationObserved => "file.mutation.observed",
            Self::ReviewDecisionRecorded => "review.decision.recorded",
            Self::VerificationCompleted => "verification.completed",
            Self::UsageObserved => "usage.observed",
            Self::OperationFailed => "operation.failed",
            Self::CoverageGapRecorded => "coverage.gap.recorded",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "run.started" => Self::RunStarted,
            "run.completed" => Self::RunCompleted,
            "agent.delegated" => Self::AgentDelegated,
            "agent.completed" => Self::AgentCompleted,
            "tool.started" => Self::ToolStarted,
            "tool.completed" => Self::ToolCompleted,
            "command.started" => Self::CommandStarted,
            "command.completed" => Self::CommandCompleted,
            "shell.opened" => Self::ShellOpened,
            "shell.closed" => Self::ShellClosed,
            "file.mutation.observed" => Self::FileMutationObserved,
            "review.decision.recorded" => Self::ReviewDecisionRecorded,
            "verification.completed" => Self::VerificationCompleted,
            "usage.observed" => Self::UsageObserved,
            "operation.failed" => Self::OperationFailed,
            "coverage.gap.recorded" => Self::CoverageGapRecorded,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvidenceOutcome {
    Succeeded,
    Failed,
    Cancelled,
    Incomplete,
}

impl EvidenceOutcome {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Incomplete => "incomplete",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandRuntimeKind {
    LocalShell,
    RemoteShell,
    Process,
    Unknown,
}

impl CommandRuntimeKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::LocalShell => "local-shell",
            Self::RemoteShell => "remote-shell",
            Self::Process => "process",
            Self::Unknown => "unknown",
        }
    }
}

/// What the runtime could observe about the output, not what it wishes it had. `Merged` exists so
/// a PTY-observed command is never presented as if stdout and stderr had been separated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputAvailability {
    Merged,
    Separate,
    Unavailable,
    Redacted,
}

impl OutputAvailability {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Merged => "merged",
            Self::Separate => "separate",
            Self::Unavailable => "unavailable",
            Self::Redacted => "redacted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
}

impl FileChangeKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Modified => "modified",
            Self::Deleted => "deleted",
            Self::Renamed => "renamed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReviewDecisionScope {
    Review,
    Hunk,
    FileViewed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReviewDecisionValue {
    Pending,
    Accepted,
    ChangesRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerificationOutcome {
    Passed,
    Failed,
    Skipped,
    Unknown,
}

impl VerificationOutcome {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Unknown => "unknown",
        }
    }
}

/// Which accounting quality a usage observation had. Evidence keeps the classification and the
/// reference; the token dimensions stay in the sessions usage read model that owns them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UsageQuality {
    Reported,
    ReportedDerived,
    Estimated,
}

/// The complete set of things evidence is allowed to say.
///
/// This enum is the privacy boundary. It is exhaustive and versioned by design: there is no
/// `Custom`, no attribute map, and no `serde_json::Value`, so a producer has no channel through
/// which a prompt, a model response, a raw tool argument, a terminal transcript, a diff, or a
/// header could arrive — not because a filter would catch it, but because no variant can hold it.
/// Adding a semantic kind therefore means adding a variant, a schema version, and tests, which is
/// the review point this design wanted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SafeEvidencePayload {
    RunStarted {
        trigger: SafeReasonCode,
    },
    RunCompleted {
        outcome: EvidenceOutcome,
        duration_ms: Option<u64>,
    },
    AgentDelegated {
        attempt: Option<u32>,
    },
    AgentCompleted {
        outcome: EvidenceOutcome,
        duration_ms: Option<u64>,
    },
    ToolStarted {
        tool_name: BoundedLabel,
    },
    ToolCompleted {
        tool_name: BoundedLabel,
        outcome: EvidenceOutcome,
        duration_ms: Option<u64>,
    },
    CommandStarted {
        runtime_kind: CommandRuntimeKind,
        redacted_display: Option<RedactedCommandDisplay>,
        cwd_display: Option<RelativeDisplayPath>,
    },
    CommandCompleted {
        outcome: EvidenceOutcome,
        duration_ms: Option<u64>,
        exit_code: Option<i32>,
        signal: Option<BoundedLabel>,
        output_availability: OutputAvailability,
        output_truncated: bool,
    },
    ShellOpened {
        runtime_kind: CommandRuntimeKind,
    },
    ShellClosed {
        reason: SafeReasonCode,
    },
    FileMutationObserved {
        basename: SafeBasename,
        path_fingerprint: SafeFingerprint,
        change_kind: FileChangeKind,
    },
    ReviewDecisionRecorded {
        scope: ReviewDecisionScope,
        decision: ReviewDecisionValue,
    },
    VerificationCompleted {
        name: BoundedLabel,
        outcome: VerificationOutcome,
        passed_count: Option<u32>,
        failed_count: Option<u32>,
    },
    UsageObserved {
        quality: UsageQuality,
        response_count: u32,
    },
    OperationFailed {
        reason: SafeReasonCode,
    },
    CoverageGapRecorded {
        dropped_count: u32,
        reason: SafeReasonCode,
    },
}

impl SafeEvidencePayload {
    /// The one kind this payload can legally be filed under. Deriving it here rather than trusting
    /// a separately supplied kind is what makes a mismatch impossible to persist.
    pub(crate) fn kind(&self) -> EvidenceKind {
        match self {
            Self::RunStarted { .. } => EvidenceKind::RunStarted,
            Self::RunCompleted { .. } => EvidenceKind::RunCompleted,
            Self::AgentDelegated { .. } => EvidenceKind::AgentDelegated,
            Self::AgentCompleted { .. } => EvidenceKind::AgentCompleted,
            Self::ToolStarted { .. } => EvidenceKind::ToolStarted,
            Self::ToolCompleted { .. } => EvidenceKind::ToolCompleted,
            Self::CommandStarted { .. } => EvidenceKind::CommandStarted,
            Self::CommandCompleted { .. } => EvidenceKind::CommandCompleted,
            Self::ShellOpened { .. } => EvidenceKind::ShellOpened,
            Self::ShellClosed { .. } => EvidenceKind::ShellClosed,
            Self::FileMutationObserved { .. } => EvidenceKind::FileMutationObserved,
            Self::ReviewDecisionRecorded { .. } => EvidenceKind::ReviewDecisionRecorded,
            Self::VerificationCompleted { .. } => EvidenceKind::VerificationCompleted,
            Self::UsageObserved { .. } => EvidenceKind::UsageObserved,
            Self::OperationFailed { .. } => EvidenceKind::OperationFailed,
            Self::CoverageGapRecorded { .. } => EvidenceKind::CoverageGapRecorded,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), EvidenceDomainError> {
        // A gap that dropped nothing is not a gap; recording one would manufacture the very
        // incompleteness the marker exists to report.
        if let Self::CoverageGapRecorded { dropped_count, .. } = self {
            if *dropped_count == 0 {
                return Err(EvidenceDomainError::EmptyCoverageGap);
            }
        }
        Ok(())
    }
}
