use crate::contexts::skill_evolution_generation::application::CanonicalEncodingError;
use crate::contexts::skill_evolution_generation::domain::{
    GenerationJobStatus, GenerationStageKind, GenerationStageStatus, GenerationUsageV1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationPersistenceError {
    InvalidInput,
    Conflict,
    Immutable,
    Storage,
}

impl From<CanonicalEncodingError> for GenerationPersistenceError {
    fn from(_: CanonicalEncodingError) -> Self {
        Self::InvalidInput
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PersistGenerationOutcome {
    Inserted { id: String },
    Coalesced { id: String },
}

pub(crate) struct JobTransition<'a> {
    pub(crate) job_id: &'a str,
    pub(crate) expected_revision: u64,
    pub(crate) status: GenerationJobStatus,
    pub(crate) current_stage: Option<GenerationStageKind>,
    pub(crate) usage_json: &'a str,
    pub(crate) safe_failure_code: Option<&'a str>,
    pub(crate) updated_at_ms: i64,
}

pub(crate) struct StageAttemptCompletion<'a> {
    pub(crate) attempt_id: &'a str,
    pub(crate) status: GenerationStageStatus,
    pub(crate) expected_input_hash: &'a str,
    pub(crate) output_hash: Option<&'a str>,
    pub(crate) usage: &'a GenerationUsageV1,
    pub(crate) safe_failure_code: Option<&'a str>,
    pub(crate) completed_at_ms: i64,
    pub(crate) superseded_by_attempt_id: Option<&'a str>,
}

pub(crate) fn job_status_name(status: GenerationJobStatus) -> &'static str {
    match status {
        GenerationJobStatus::Requested => "requested",
        GenerationJobStatus::BlockedConsent => "blocked_consent",
        GenerationJobStatus::Queued => "queued",
        GenerationJobStatus::Running => "running",
        GenerationJobStatus::CancelRequested => "cancel_requested",
        GenerationJobStatus::Cancelled => "cancelled",
        GenerationJobStatus::Failed => "failed",
        GenerationJobStatus::Completed => "completed",
        GenerationJobStatus::Superseded => "superseded",
    }
}

pub(crate) fn stage_name(stage: GenerationStageKind) -> &'static str {
    match stage {
        GenerationStageKind::FreezeInput => "freeze_input",
        GenerationStageKind::InspectTarget => "inspect_target",
        GenerationStageKind::BuildDossier => "build_dossier",
        GenerationStageKind::PlanMutation => "plan_mutation",
        GenerationStageKind::SynthesizeStructuredDraft => "synthesize_structured_draft",
        GenerationStageKind::ValidateAndSimulate => "validate_and_simulate",
        GenerationStageKind::PackageForGovernance => "package_for_governance",
    }
}
