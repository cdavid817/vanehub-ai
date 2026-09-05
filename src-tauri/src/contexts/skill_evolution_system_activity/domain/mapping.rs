use super::mapping_lifecycle::*;
use super::*;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MappedActivityOutcome {
    pub(crate) event_code: ActivityEventCode,
    pub(crate) severity: ActivitySeverity,
    pub(crate) status: ActivityStatus,
    pub(crate) attention_kind: ActivityAttentionKind,
    pub(crate) reason_codes: Vec<ActivityReasonCode>,
    pub(crate) payload: ActivityPayloadV1,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum ActivityMappingError {
    #[error("source outcome is not registered for activity projection")]
    UnsupportedOutcome,
}

pub(crate) fn map_source_outcome(
    domain: EvolutionSourceDomain,
    source_kind: &str,
    outcome: &str,
) -> Result<MappedActivityOutcome, ActivityMappingError> {
    match domain {
        EvolutionSourceDomain::Orchestration => map_orchestration(source_kind, outcome),
        EvolutionSourceDomain::Evidence => map_evidence(source_kind, outcome),
        EvolutionSourceDomain::Assessment => map_assessment(outcome),
        EvolutionSourceDomain::Generation => map_generation(source_kind, outcome),
        EvolutionSourceDomain::Curator => map_curator(outcome),
        EvolutionSourceDomain::Overlay => map_overlay(outcome),
        EvolutionSourceDomain::AutomaticApplication => map_automatic(source_kind, outcome),
        EvolutionSourceDomain::Probation => map_probation(outcome),
        EvolutionSourceDomain::Breaker => map_breaker(outcome),
        EvolutionSourceDomain::SkillCreation => map_skill_creation(outcome),
        EvolutionSourceDomain::Recovery => map_recovery(outcome),
        EvolutionSourceDomain::Retention => map_retention(outcome),
    }
}

fn map_orchestration(
    source_kind: &str,
    outcome: &str,
) -> Result<MappedActivityOutcome, ActivityMappingError> {
    match (source_kind, outcome) {
        ("run", "requested" | "waiting_idle" | "running") => Ok(mapped(
            ActivityEventCode::RunStarted,
            ActivitySeverity::Info,
            ActivityStatus::Running,
            ActivityAttentionKind::None,
            ActivityReasonCode::Started,
            ActivityValueCode::Running,
        )),
        ("run", "partial") => Ok(mapped(
            ActivityEventCode::RunCompleted,
            ActivitySeverity::Warning,
            ActivityStatus::Succeeded,
            ActivityAttentionKind::Review,
            ActivityReasonCode::Partial,
            ActivityValueCode::Completed,
        )),
        ("run", "completed") => Ok(success(
            ActivityEventCode::RunCompleted,
            ActivityValueCode::Completed,
        )),
        ("run", "failed") => Ok(failure(ActivityEventCode::RunFailed)),
        ("run", "cancel_requested" | "cancelled") => Ok(cancelled(ActivityEventCode::RunFailed)),
        ("run", "recovered") => map_recovery("completed"),
        ("stage", "pending" | "running") => Ok(mapped(
            ActivityEventCode::StageStarted,
            ActivitySeverity::Info,
            ActivityStatus::Running,
            ActivityAttentionKind::None,
            ActivityReasonCode::Started,
            ActivityValueCode::Running,
        )),
        ("stage", "completed" | "succeeded") => Ok(success(
            ActivityEventCode::StageCompleted,
            ActivityValueCode::Completed,
        )),
        ("stage", "failed") => Ok(failure(ActivityEventCode::StageFailed)),
        _ => Err(ActivityMappingError::UnsupportedOutcome),
    }
}

fn map_evidence(
    source_kind: &str,
    outcome: &str,
) -> Result<MappedActivityOutcome, ActivityMappingError> {
    match (source_kind, outcome) {
        ("signal", "ingested") => Ok(success(
            ActivityEventCode::EvidenceReady,
            ActivityValueCode::Ready,
        )),
        ("seed", "ready") => Ok(success(
            ActivityEventCode::SeedReady,
            ActivityValueCode::Ready,
        )),
        ("seed", "human_review_only") => Ok(mapped(
            ActivityEventCode::SeedReady,
            ActivitySeverity::Warning,
            ActivityStatus::Blocked,
            ActivityAttentionKind::Review,
            ActivityReasonCode::ReviewRequired,
            ActivityValueCode::Blocked,
        )),
        _ => Err(ActivityMappingError::UnsupportedOutcome),
    }
}

fn map_assessment(outcome: &str) -> Result<MappedActivityOutcome, ActivityMappingError> {
    match outcome {
        "completed" => Ok(success(
            ActivityEventCode::AssessmentCompleted,
            ActivityValueCode::Completed,
        )),
        "review" => Ok(mapped(
            ActivityEventCode::AssessmentNeedsReview,
            ActivitySeverity::Warning,
            ActivityStatus::Blocked,
            ActivityAttentionKind::Review,
            ActivityReasonCode::ReviewRequired,
            ActivityValueCode::Blocked,
        )),
        "failed" => Ok(failure(ActivityEventCode::AssessmentCompleted)),
        _ => Err(ActivityMappingError::UnsupportedOutcome),
    }
}

fn map_generation(
    source_kind: &str,
    outcome: &str,
) -> Result<MappedActivityOutcome, ActivityMappingError> {
    match (source_kind, outcome) {
        ("job", "requested" | "queued" | "running") => Ok(mapped(
            ActivityEventCode::GenerationStarted,
            ActivitySeverity::Info,
            ActivityStatus::Running,
            ActivityAttentionKind::None,
            ActivityReasonCode::Started,
            ActivityValueCode::Running,
        )),
        ("job", "completed") => Ok(success(
            ActivityEventCode::GenerationCompleted,
            ActivityValueCode::Completed,
        )),
        ("job", "failed") => Ok(failure(ActivityEventCode::GenerationFailed)),
        ("job", "cancel_requested" | "cancelled") => {
            Ok(cancelled(ActivityEventCode::GenerationFailed))
        }
        ("dossier", "created") => Ok(success(
            ActivityEventCode::DossierCompleted,
            ActivityValueCode::Created,
        )),
        _ => Err(ActivityMappingError::UnsupportedOutcome),
    }
}

pub(super) fn success(code: ActivityEventCode, value: ActivityValueCode) -> MappedActivityOutcome {
    mapped(
        code,
        ActivitySeverity::Info,
        ActivityStatus::Succeeded,
        ActivityAttentionKind::None,
        ActivityReasonCode::Completed,
        value,
    )
}

fn failure(code: ActivityEventCode) -> MappedActivityOutcome {
    mapped(
        code,
        ActivitySeverity::Error,
        ActivityStatus::Failed,
        ActivityAttentionKind::Review,
        ActivityReasonCode::Failed,
        ActivityValueCode::Failed,
    )
}

fn cancelled(code: ActivityEventCode) -> MappedActivityOutcome {
    mapped(
        code,
        ActivitySeverity::Info,
        ActivityStatus::Cancelled,
        ActivityAttentionKind::None,
        ActivityReasonCode::Cancelled,
        ActivityValueCode::Cancelled,
    )
}

pub(super) fn mapped(
    event_code: ActivityEventCode,
    severity: ActivitySeverity,
    status: ActivityStatus,
    attention_kind: ActivityAttentionKind,
    reason_code: ActivityReasonCode,
    value_code: ActivityValueCode,
) -> MappedActivityOutcome {
    MappedActivityOutcome {
        event_code,
        severity,
        status,
        attention_kind,
        reason_codes: vec![reason_code],
        payload: ActivityPayloadV1::StatusCard {
            label_code: ActivityLabelCode::Outcome,
            value_code,
        },
    }
}
