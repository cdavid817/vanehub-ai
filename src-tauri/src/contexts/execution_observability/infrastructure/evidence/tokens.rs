use crate::contexts::execution_observability::domain::{
    CommandRuntimeKind, OutputAvailability, VerificationOutcome,
};

/// Column readers for the projection's enum-shaped text.
///
/// An absent or unreadable value becomes the enum's explicit "we do not know" variant rather than
/// a plausible default. `Unknown` runtime, `Unavailable` output, and `Unknown` verification each
/// already mean "not observed", so degrading into them keeps the row honest instead of implying an
/// observation the projection never made.
pub(super) fn runtime_kind(value: Option<&str>) -> CommandRuntimeKind {
    match value {
        Some("local-shell") => CommandRuntimeKind::LocalShell,
        Some("remote-shell") => CommandRuntimeKind::RemoteShell,
        Some("process") => CommandRuntimeKind::Process,
        _ => CommandRuntimeKind::Unknown,
    }
}

pub(super) fn output_availability(value: Option<&str>) -> OutputAvailability {
    match value {
        Some("merged") => OutputAvailability::Merged,
        Some("separate") => OutputAvailability::Separate,
        Some("redacted") => OutputAvailability::Redacted,
        _ => OutputAvailability::Unavailable,
    }
}

pub(super) fn verification_outcome(value: Option<&str>) -> VerificationOutcome {
    match value {
        Some("passed") => VerificationOutcome::Passed,
        Some("failed") => VerificationOutcome::Failed,
        Some("skipped") => VerificationOutcome::Skipped,
        _ => VerificationOutcome::Unknown,
    }
}
