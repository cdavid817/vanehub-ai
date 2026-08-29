use super::identity::{BoundedLabel, SafeFingerprint, SafeReasonCode};
use super::payload::{
    CommandRuntimeKind, FileChangeKind, ReviewDecisionScope, ReviewDecisionValue,
    SafeEvidencePayload, UsageQuality, VerificationOutcome,
};
use super::safety::{RedactedCommandDisplay, RelativeDisplayPath, SafeBasename};

/// A deterministic rendering of a payload.
///
/// Two properties matter and neither is negotiable. Field order is fixed by the match arms rather
/// than by a map iteration, so the same payload always encodes to the same bytes on every platform
/// and every run — a fingerprint that depended on hash order would turn retries into conflicts at
/// random. And every variant enumerates its fields explicitly, so adding a field to a variant
/// stops compiling here until someone decides where it belongs in the canonical form.
pub(crate) fn canonical_payload_encoding(payload: &SafeEvidencePayload) -> String {
    let fields: Vec<(&'static str, String)> = match payload {
        SafeEvidencePayload::RunStarted { trigger } => {
            vec![("trigger", reason(trigger))]
        }
        SafeEvidencePayload::RunCompleted {
            outcome,
            duration_ms,
        } => vec![
            ("outcome", outcome.as_str().to_string()),
            ("duration_ms", number(*duration_ms)),
        ],
        SafeEvidencePayload::AgentDelegated { attempt } => {
            vec![("attempt", number(*attempt))]
        }
        SafeEvidencePayload::AgentCompleted {
            outcome,
            duration_ms,
        } => vec![
            ("outcome", outcome.as_str().to_string()),
            ("duration_ms", number(*duration_ms)),
        ],
        SafeEvidencePayload::ToolStarted { tool_name } => {
            vec![("tool_name", label(tool_name))]
        }
        SafeEvidencePayload::ToolCompleted {
            tool_name,
            outcome,
            duration_ms,
        } => vec![
            ("tool_name", label(tool_name)),
            ("outcome", outcome.as_str().to_string()),
            ("duration_ms", number(*duration_ms)),
        ],
        SafeEvidencePayload::CommandStarted {
            runtime_kind,
            redacted_display,
            cwd_display,
        } => vec![
            ("runtime_kind", runtime(*runtime_kind)),
            (
                "redacted_display",
                optional(
                    redacted_display
                        .as_ref()
                        .map(RedactedCommandDisplay::as_str),
                ),
            ),
            (
                "cwd_display",
                optional(cwd_display.as_ref().map(RelativeDisplayPath::as_str)),
            ),
        ],
        SafeEvidencePayload::CommandCompleted {
            outcome,
            duration_ms,
            exit_code,
            signal,
            output_availability,
            output_truncated,
        } => vec![
            ("outcome", outcome.as_str().to_string()),
            ("duration_ms", number(*duration_ms)),
            ("exit_code", number(*exit_code)),
            (
                "signal",
                optional(signal.as_ref().map(BoundedLabel::as_str)),
            ),
            (
                "output_availability",
                output_availability.as_str().to_string(),
            ),
            ("output_truncated", output_truncated.to_string()),
        ],
        SafeEvidencePayload::ShellOpened { runtime_kind } => {
            vec![("runtime_kind", runtime(*runtime_kind))]
        }
        SafeEvidencePayload::ShellClosed { reason: value } => {
            vec![("reason", reason(value))]
        }
        SafeEvidencePayload::FileMutationObserved {
            basename,
            path_fingerprint,
            change_kind,
        } => vec![
            ("basename", SafeBasename::as_str(basename).to_string()),
            (
                "path_fingerprint",
                SafeFingerprint::as_str(path_fingerprint).to_string(),
            ),
            ("change_kind", change(*change_kind)),
        ],
        SafeEvidencePayload::ReviewDecisionRecorded { scope, decision } => vec![
            ("scope", review_scope(*scope).to_string()),
            ("decision", review_decision(*decision).to_string()),
        ],
        SafeEvidencePayload::VerificationCompleted {
            name,
            outcome,
            passed_count,
            failed_count,
        } => vec![
            ("name", label(name)),
            ("outcome", verification(*outcome).to_string()),
            ("passed_count", number(*passed_count)),
            ("failed_count", number(*failed_count)),
        ],
        SafeEvidencePayload::UsageObserved {
            quality,
            response_count,
        } => vec![
            ("quality", usage(*quality).to_string()),
            ("response_count", response_count.to_string()),
        ],
        SafeEvidencePayload::OperationFailed { reason: value } => {
            vec![("reason", reason(value))]
        }
        SafeEvidencePayload::CoverageGapRecorded {
            dropped_count,
            reason: value,
        } => vec![
            ("dropped_count", dropped_count.to_string()),
            ("reason", reason(value)),
        ],
    };

    let mut encoded = payload.kind().as_str().to_string();
    for (key, value) in fields {
        encoded.push('\u{1e}');
        encoded.push_str(key);
        encoded.push('=');
        encoded.push_str(&value.replace('\u{1e}', "\\u001e"));
    }
    encoded
}

/// An unobserved value encodes as empty and a zero encodes as "0", so "we did not see it" and
/// "we saw zero" never collide in the fingerprint.
fn number<T: ToString>(value: Option<T>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn optional(value: Option<&str>) -> String {
    value.unwrap_or_default().to_string()
}

fn reason(value: &SafeReasonCode) -> String {
    value.as_str().to_string()
}

fn label(value: &BoundedLabel) -> String {
    value.as_str().to_string()
}

fn runtime(value: CommandRuntimeKind) -> String {
    value.as_str().to_string()
}

fn change(value: FileChangeKind) -> String {
    value.as_str().to_string()
}

pub(crate) fn review_scope(scope: ReviewDecisionScope) -> &'static str {
    match scope {
        ReviewDecisionScope::Review => "review",
        ReviewDecisionScope::Hunk => "hunk",
        ReviewDecisionScope::FileViewed => "file_viewed",
    }
}

pub(crate) fn review_decision(decision: ReviewDecisionValue) -> &'static str {
    match decision {
        ReviewDecisionValue::Pending => "pending",
        ReviewDecisionValue::Accepted => "accepted",
        ReviewDecisionValue::ChangesRequested => "changes_requested",
    }
}

pub(crate) fn verification(outcome: VerificationOutcome) -> &'static str {
    match outcome {
        VerificationOutcome::Passed => "passed",
        VerificationOutcome::Failed => "failed",
        VerificationOutcome::Skipped => "skipped",
        VerificationOutcome::Unknown => "unknown",
    }
}

pub(crate) fn usage(quality: UsageQuality) -> &'static str {
    match quality {
        UsageQuality::Reported => "reported",
        UsageQuality::ReportedDerived => "reported_derived",
        UsageQuality::Estimated => "estimated",
    }
}
