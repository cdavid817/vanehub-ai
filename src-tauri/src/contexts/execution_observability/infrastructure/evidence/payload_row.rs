use crate::contexts::execution_observability::domain::evidence::identity::{
    BoundedLabel, SafeFingerprint, SafeReasonCode,
};
use crate::contexts::execution_observability::domain::evidence::payload::{
    CommandRuntimeKind, EvidenceOutcome, FileChangeKind, OutputAvailability, ReviewDecisionScope,
    ReviewDecisionValue, SafeEvidencePayload, UsageQuality, VerificationOutcome,
};
use crate::contexts::execution_observability::domain::evidence::safety::{
    RedactedCommandDisplay, RelativeDisplayPath, SafeBasename,
};
use crate::contexts::execution_observability::domain::EvidenceDomainError;
use serde::{Deserialize, Serialize};

/// The storage mirror of the payload enum.
///
/// Serde lives here rather than on the domain type on purpose. The domain's newtypes exist because
/// they cannot be constructed without passing validation, and a `Deserialize` derived on them
/// would hand back a validated-looking value that never was. Reading therefore goes through the
/// domain constructors, so a row edited by hand, written by an older build, or corrupted on disk
/// fails to load instead of re-entering the system as trusted evidence.
///
/// The tag is explicit and closed: an unknown variant is a parse error, not a silently dropped row.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub(super) enum StoredPayload {
    #[serde(rename = "run.started")]
    RunStarted { trigger: String },
    #[serde(rename = "run.completed")]
    RunCompleted {
        outcome: String,
        duration_ms: Option<u64>,
    },
    #[serde(rename = "agent.delegated")]
    AgentDelegated { attempt: Option<u32> },
    #[serde(rename = "agent.completed")]
    AgentCompleted {
        outcome: String,
        duration_ms: Option<u64>,
    },
    #[serde(rename = "tool.started")]
    ToolStarted { tool_name: String },
    #[serde(rename = "tool.completed")]
    ToolCompleted {
        tool_name: String,
        outcome: String,
        duration_ms: Option<u64>,
    },
    #[serde(rename = "command.started")]
    CommandStarted {
        runtime_kind: String,
        redacted_display: Option<String>,
        cwd_display: Option<String>,
    },
    #[serde(rename = "command.completed")]
    CommandCompleted {
        outcome: String,
        duration_ms: Option<u64>,
        exit_code: Option<i32>,
        signal: Option<String>,
        output_availability: String,
        output_truncated: bool,
    },
    #[serde(rename = "shell.opened")]
    ShellOpened { runtime_kind: String },
    #[serde(rename = "shell.closed")]
    ShellClosed { reason: String },
    #[serde(rename = "file.mutation.observed")]
    FileMutationObserved {
        basename: String,
        path_fingerprint: String,
        change_kind: String,
    },
    #[serde(rename = "review.decision.recorded")]
    ReviewDecisionRecorded { scope: String, decision: String },
    #[serde(rename = "verification.completed")]
    VerificationCompleted {
        name: String,
        outcome: String,
        passed_count: Option<u32>,
        failed_count: Option<u32>,
    },
    #[serde(rename = "usage.observed")]
    UsageObserved {
        quality: String,
        response_count: u32,
    },
    #[serde(rename = "operation.failed")]
    OperationFailed { reason: String },
    #[serde(rename = "coverage.gap.recorded")]
    CoverageGapRecorded { dropped_count: u32, reason: String },
}

pub(super) fn to_stored(payload: &SafeEvidencePayload) -> StoredPayload {
    match payload {
        SafeEvidencePayload::RunStarted { trigger } => StoredPayload::RunStarted {
            trigger: trigger.as_str().to_string(),
        },
        SafeEvidencePayload::RunCompleted {
            outcome,
            duration_ms,
        } => StoredPayload::RunCompleted {
            outcome: outcome.as_str().to_string(),
            duration_ms: *duration_ms,
        },
        SafeEvidencePayload::AgentDelegated { attempt } => {
            StoredPayload::AgentDelegated { attempt: *attempt }
        }
        SafeEvidencePayload::AgentCompleted {
            outcome,
            duration_ms,
        } => StoredPayload::AgentCompleted {
            outcome: outcome.as_str().to_string(),
            duration_ms: *duration_ms,
        },
        SafeEvidencePayload::ToolStarted { tool_name } => StoredPayload::ToolStarted {
            tool_name: tool_name.as_str().to_string(),
        },
        SafeEvidencePayload::ToolCompleted {
            tool_name,
            outcome,
            duration_ms,
        } => StoredPayload::ToolCompleted {
            tool_name: tool_name.as_str().to_string(),
            outcome: outcome.as_str().to_string(),
            duration_ms: *duration_ms,
        },
        SafeEvidencePayload::CommandStarted {
            runtime_kind,
            redacted_display,
            cwd_display,
        } => StoredPayload::CommandStarted {
            runtime_kind: runtime_kind.as_str().to_string(),
            redacted_display: redacted_display
                .as_ref()
                .map(|value| value.as_str().to_string()),
            cwd_display: cwd_display.as_ref().map(|value| value.as_str().to_string()),
        },
        SafeEvidencePayload::CommandCompleted {
            outcome,
            duration_ms,
            exit_code,
            signal,
            output_availability,
            output_truncated,
        } => StoredPayload::CommandCompleted {
            outcome: outcome.as_str().to_string(),
            duration_ms: *duration_ms,
            exit_code: *exit_code,
            signal: signal.as_ref().map(|value| value.as_str().to_string()),
            output_availability: output_availability.as_str().to_string(),
            output_truncated: *output_truncated,
        },
        SafeEvidencePayload::ShellOpened { runtime_kind } => StoredPayload::ShellOpened {
            runtime_kind: runtime_kind.as_str().to_string(),
        },
        SafeEvidencePayload::ShellClosed { reason } => StoredPayload::ShellClosed {
            reason: reason.as_str().to_string(),
        },
        SafeEvidencePayload::FileMutationObserved {
            basename,
            path_fingerprint,
            change_kind,
        } => StoredPayload::FileMutationObserved {
            basename: basename.as_str().to_string(),
            path_fingerprint: path_fingerprint.as_str().to_string(),
            change_kind: change_kind.as_str().to_string(),
        },
        SafeEvidencePayload::ReviewDecisionRecorded { scope, decision } => {
            StoredPayload::ReviewDecisionRecorded {
                scope: review_scope_token(*scope).to_string(),
                decision: review_decision_token(*decision).to_string(),
            }
        }
        SafeEvidencePayload::VerificationCompleted {
            name,
            outcome,
            passed_count,
            failed_count,
        } => StoredPayload::VerificationCompleted {
            name: name.as_str().to_string(),
            outcome: verification_token(*outcome).to_string(),
            passed_count: *passed_count,
            failed_count: *failed_count,
        },
        SafeEvidencePayload::UsageObserved {
            quality,
            response_count,
        } => StoredPayload::UsageObserved {
            quality: usage_token(*quality).to_string(),
            response_count: *response_count,
        },
        SafeEvidencePayload::OperationFailed { reason } => StoredPayload::OperationFailed {
            reason: reason.as_str().to_string(),
        },
        SafeEvidencePayload::CoverageGapRecorded {
            dropped_count,
            reason,
        } => StoredPayload::CoverageGapRecorded {
            dropped_count: *dropped_count,
            reason: reason.as_str().to_string(),
        },
    }
}

pub(super) fn from_stored(
    stored: StoredPayload,
) -> Result<SafeEvidencePayload, EvidenceDomainError> {
    let unknown = || EvidenceDomainError::PayloadKindMismatch { kind: "stored" };
    Ok(match stored {
        StoredPayload::RunStarted { trigger } => SafeEvidencePayload::RunStarted {
            trigger: SafeReasonCode::parse(trigger)?,
        },
        StoredPayload::RunCompleted {
            outcome,
            duration_ms,
        } => SafeEvidencePayload::RunCompleted {
            outcome: outcome_from(&outcome).ok_or_else(unknown)?,
            duration_ms,
        },
        StoredPayload::AgentDelegated { attempt } => {
            SafeEvidencePayload::AgentDelegated { attempt }
        }
        StoredPayload::AgentCompleted {
            outcome,
            duration_ms,
        } => SafeEvidencePayload::AgentCompleted {
            outcome: outcome_from(&outcome).ok_or_else(unknown)?,
            duration_ms,
        },
        StoredPayload::ToolStarted { tool_name } => SafeEvidencePayload::ToolStarted {
            tool_name: BoundedLabel::parse("tool name", tool_name)?,
        },
        StoredPayload::ToolCompleted {
            tool_name,
            outcome,
            duration_ms,
        } => SafeEvidencePayload::ToolCompleted {
            tool_name: BoundedLabel::parse("tool name", tool_name)?,
            outcome: outcome_from(&outcome).ok_or_else(unknown)?,
            duration_ms,
        },
        StoredPayload::CommandStarted {
            runtime_kind,
            redacted_display,
            cwd_display,
        } => SafeEvidencePayload::CommandStarted {
            runtime_kind: runtime_from(&runtime_kind).ok_or_else(unknown)?,
            redacted_display: redacted_display
                .map(RedactedCommandDisplay::parse)
                .transpose()?,
            cwd_display: cwd_display.map(RelativeDisplayPath::parse).transpose()?,
        },
        StoredPayload::CommandCompleted {
            outcome,
            duration_ms,
            exit_code,
            signal,
            output_availability,
            output_truncated,
        } => SafeEvidencePayload::CommandCompleted {
            outcome: outcome_from(&outcome).ok_or_else(unknown)?,
            duration_ms,
            exit_code,
            signal: signal
                .map(|value| BoundedLabel::parse("signal", value))
                .transpose()?,
            output_availability: output_from(&output_availability).ok_or_else(unknown)?,
            output_truncated,
        },
        StoredPayload::ShellOpened { runtime_kind } => SafeEvidencePayload::ShellOpened {
            runtime_kind: runtime_from(&runtime_kind).ok_or_else(unknown)?,
        },
        StoredPayload::ShellClosed { reason } => SafeEvidencePayload::ShellClosed {
            reason: SafeReasonCode::parse(reason)?,
        },
        StoredPayload::FileMutationObserved {
            basename,
            path_fingerprint,
            change_kind,
        } => SafeEvidencePayload::FileMutationObserved {
            basename: SafeBasename::parse(basename)?,
            path_fingerprint: SafeFingerprint::parse(path_fingerprint)?,
            change_kind: change_from(&change_kind).ok_or_else(unknown)?,
        },
        StoredPayload::ReviewDecisionRecorded { scope, decision } => {
            SafeEvidencePayload::ReviewDecisionRecorded {
                scope: review_scope_from(&scope).ok_or_else(unknown)?,
                decision: review_decision_from(&decision).ok_or_else(unknown)?,
            }
        }
        StoredPayload::VerificationCompleted {
            name,
            outcome,
            passed_count,
            failed_count,
        } => SafeEvidencePayload::VerificationCompleted {
            name: BoundedLabel::parse("verification name", name)?,
            outcome: verification_from(&outcome).ok_or_else(unknown)?,
            passed_count,
            failed_count,
        },
        StoredPayload::UsageObserved {
            quality,
            response_count,
        } => SafeEvidencePayload::UsageObserved {
            quality: usage_from(&quality).ok_or_else(unknown)?,
            response_count,
        },
        StoredPayload::OperationFailed { reason } => SafeEvidencePayload::OperationFailed {
            reason: SafeReasonCode::parse(reason)?,
        },
        StoredPayload::CoverageGapRecorded {
            dropped_count,
            reason,
        } => SafeEvidencePayload::CoverageGapRecorded {
            dropped_count,
            reason: SafeReasonCode::parse(reason)?,
        },
    })
}

fn outcome_from(value: &str) -> Option<EvidenceOutcome> {
    Some(match value {
        "succeeded" => EvidenceOutcome::Succeeded,
        "failed" => EvidenceOutcome::Failed,
        "cancelled" => EvidenceOutcome::Cancelled,
        "incomplete" => EvidenceOutcome::Incomplete,
        _ => return None,
    })
}

fn runtime_from(value: &str) -> Option<CommandRuntimeKind> {
    Some(match value {
        "local-shell" => CommandRuntimeKind::LocalShell,
        "remote-shell" => CommandRuntimeKind::RemoteShell,
        "process" => CommandRuntimeKind::Process,
        "unknown" => CommandRuntimeKind::Unknown,
        _ => return None,
    })
}

fn output_from(value: &str) -> Option<OutputAvailability> {
    Some(match value {
        "merged" => OutputAvailability::Merged,
        "separate" => OutputAvailability::Separate,
        "unavailable" => OutputAvailability::Unavailable,
        "redacted" => OutputAvailability::Redacted,
        _ => return None,
    })
}

fn change_from(value: &str) -> Option<FileChangeKind> {
    Some(match value {
        "added" => FileChangeKind::Added,
        "modified" => FileChangeKind::Modified,
        "deleted" => FileChangeKind::Deleted,
        "renamed" => FileChangeKind::Renamed,
        _ => return None,
    })
}

pub(super) fn review_scope_token(scope: ReviewDecisionScope) -> &'static str {
    match scope {
        ReviewDecisionScope::Review => "review",
        ReviewDecisionScope::Hunk => "hunk",
        ReviewDecisionScope::FileViewed => "file_viewed",
    }
}

fn review_scope_from(value: &str) -> Option<ReviewDecisionScope> {
    Some(match value {
        "review" => ReviewDecisionScope::Review,
        "hunk" => ReviewDecisionScope::Hunk,
        "file_viewed" => ReviewDecisionScope::FileViewed,
        _ => return None,
    })
}

pub(super) fn review_decision_token(decision: ReviewDecisionValue) -> &'static str {
    match decision {
        ReviewDecisionValue::Pending => "pending",
        ReviewDecisionValue::Accepted => "accepted",
        ReviewDecisionValue::ChangesRequested => "changes_requested",
    }
}

fn review_decision_from(value: &str) -> Option<ReviewDecisionValue> {
    Some(match value {
        "pending" => ReviewDecisionValue::Pending,
        "accepted" => ReviewDecisionValue::Accepted,
        "changes_requested" => ReviewDecisionValue::ChangesRequested,
        _ => return None,
    })
}

pub(super) fn verification_token(outcome: VerificationOutcome) -> &'static str {
    match outcome {
        VerificationOutcome::Passed => "passed",
        VerificationOutcome::Failed => "failed",
        VerificationOutcome::Skipped => "skipped",
        VerificationOutcome::Unknown => "unknown",
    }
}

pub(super) fn verification_from(value: &str) -> Option<VerificationOutcome> {
    Some(match value {
        "passed" => VerificationOutcome::Passed,
        "failed" => VerificationOutcome::Failed,
        "skipped" => VerificationOutcome::Skipped,
        "unknown" => VerificationOutcome::Unknown,
        _ => return None,
    })
}

fn usage_token(quality: UsageQuality) -> &'static str {
    match quality {
        UsageQuality::Reported => "reported",
        UsageQuality::ReportedDerived => "reported_derived",
        UsageQuality::Estimated => "estimated",
    }
}

fn usage_from(value: &str) -> Option<UsageQuality> {
    Some(match value {
        "reported" => UsageQuality::Reported,
        "reported_derived" => UsageQuality::ReportedDerived,
        "estimated" => UsageQuality::Estimated,
        _ => return None,
    })
}
