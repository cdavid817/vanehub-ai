use super::mapping::{mapped, success};
use super::*;

pub(super) fn map_curator(outcome: &str) -> Result<MappedActivityOutcome, ActivityMappingError> {
    let (code, value, reason) = match outcome {
        "queued" => (
            ActivityEventCode::CuratorQueued,
            ActivityValueCode::Pending,
            ActivityReasonCode::ReviewRequired,
        ),
        "approved" => (
            ActivityEventCode::CuratorApproved,
            ActivityValueCode::Approved,
            ActivityReasonCode::Completed,
        ),
        "rejected" => (
            ActivityEventCode::CuratorRejected,
            ActivityValueCode::Rejected,
            ActivityReasonCode::Completed,
        ),
        "deferred" => (
            ActivityEventCode::CuratorDeferred,
            ActivityValueCode::Deferred,
            ActivityReasonCode::ReviewRequired,
        ),
        _ => return Err(ActivityMappingError::UnsupportedOutcome),
    };
    Ok(mapped(
        code,
        ActivitySeverity::Info,
        ActivityStatus::Succeeded,
        ActivityAttentionKind::None,
        reason,
        value,
    ))
}

pub(super) fn map_overlay(outcome: &str) -> Result<MappedActivityOutcome, ActivityMappingError> {
    match outcome {
        "previewed" => Ok(success(
            ActivityEventCode::OverlayPreviewed,
            ActivityValueCode::Ready,
        )),
        "applied" => Ok(success(
            ActivityEventCode::OverlayApplied,
            ActivityValueCode::Applied,
        )),
        "reverted" => Ok(success(
            ActivityEventCode::OverlayReverted,
            ActivityValueCode::Reverted,
        )),
        _ => Err(ActivityMappingError::UnsupportedOutcome),
    }
}

pub(super) fn map_automatic(
    source_kind: &str,
    outcome: &str,
) -> Result<MappedActivityOutcome, ActivityMappingError> {
    match (source_kind, outcome) {
        ("eligibility", "eligible" | "would_apply") => Ok(success(
            ActivityEventCode::AutomaticEligible,
            ActivityValueCode::Eligible,
        )),
        ("eligibility", "ineligible" | "waiting" | "routed_to_curator") => Ok(mapped(
            ActivityEventCode::AutomaticBlocked,
            ActivitySeverity::Info,
            ActivityStatus::Blocked,
            ActivityAttentionKind::None,
            ActivityReasonCode::PolicyBlocked,
            ActivityValueCode::Ineligible,
        )),
        ("application", "applied") => Ok(success(
            ActivityEventCode::AutomaticApplied,
            ActivityValueCode::Applied,
        )),
        ("application", "failed") => Ok(mapped(
            ActivityEventCode::AutomaticBlocked,
            ActivitySeverity::Error,
            ActivityStatus::Failed,
            ActivityAttentionKind::ApplicationFailure,
            ActivityReasonCode::ApplicationFailed,
            ActivityValueCode::Failed,
        )),
        _ => Err(ActivityMappingError::UnsupportedOutcome),
    }
}

pub(super) fn map_probation(outcome: &str) -> Result<MappedActivityOutcome, ActivityMappingError> {
    match outcome {
        "active" => Ok(success(
            ActivityEventCode::ProbationStarted,
            ActivityValueCode::Started,
        )),
        "healthy" | "expired" => Ok(success(
            ActivityEventCode::ProbationPassed,
            ActivityValueCode::Healthy,
        )),
        "regressed" => Ok(mapped(
            ActivityEventCode::ProbationRegressed,
            ActivitySeverity::Error,
            ActivityStatus::Failed,
            ActivityAttentionKind::Regression,
            ActivityReasonCode::RegressionDetected,
            ActivityValueCode::Regressed,
        )),
        _ => Err(ActivityMappingError::UnsupportedOutcome),
    }
}

pub(super) fn map_breaker(outcome: &str) -> Result<MappedActivityOutcome, ActivityMappingError> {
    match outcome {
        "open" | "awaiting_health" | "awaiting_acknowledgement" => Ok(mapped(
            ActivityEventCode::BreakerOpened,
            ActivitySeverity::Error,
            ActivityStatus::Blocked,
            ActivityAttentionKind::Breaker,
            ActivityReasonCode::BreakerOpened,
            ActivityValueCode::Open,
        )),
        "closed" => Ok(success(
            ActivityEventCode::BreakerClosed,
            ActivityValueCode::Closed,
        )),
        _ => Err(ActivityMappingError::UnsupportedOutcome),
    }
}

pub(super) fn map_skill_creation(
    outcome: &str,
) -> Result<MappedActivityOutcome, ActivityMappingError> {
    match outcome {
        "reviewable" | "applied" => Ok(success(
            ActivityEventCode::SkillCreated,
            ActivityValueCode::Created,
        )),
        "rejected" | "purged" | "superseded" => Ok(mapped(
            ActivityEventCode::GenerationFailed,
            ActivitySeverity::Warning,
            ActivityStatus::Failed,
            ActivityAttentionKind::Review,
            ActivityReasonCode::ValidationFailed,
            ActivityValueCode::Failed,
        )),
        _ => Err(ActivityMappingError::UnsupportedOutcome),
    }
}

pub(super) fn map_recovery(outcome: &str) -> Result<MappedActivityOutcome, ActivityMappingError> {
    match outcome {
        "completed" | "reconciled" => Ok(mapped(
            ActivityEventCode::RecoveryCompleted,
            ActivitySeverity::Warning,
            ActivityStatus::Succeeded,
            ActivityAttentionKind::None,
            ActivityReasonCode::Recovered,
            ActivityValueCode::Completed,
        )),
        "failed" => Ok(mapped(
            ActivityEventCode::ReconciliationFailed,
            ActivitySeverity::Error,
            ActivityStatus::Failed,
            ActivityAttentionKind::Integrity,
            ActivityReasonCode::IntegrityFailed,
            ActivityValueCode::Failed,
        )),
        _ => Err(ActivityMappingError::UnsupportedOutcome),
    }
}

pub(super) fn map_retention(outcome: &str) -> Result<MappedActivityOutcome, ActivityMappingError> {
    match outcome {
        "applied" => Ok(success(
            ActivityEventCode::RetentionApplied,
            ActivityValueCode::Completed,
        )),
        "purged" => Ok(mapped(
            ActivityEventCode::SourcePurged,
            ActivitySeverity::Warning,
            ActivityStatus::Succeeded,
            ActivityAttentionKind::None,
            ActivityReasonCode::SourcePurged,
            ActivityValueCode::Purged,
        )),
        _ => Err(ActivityMappingError::UnsupportedOutcome),
    }
}
