use serde::Serialize;

use crate::contexts::skill_evolution_orchestration::domain::{
    is_safe_identifier, orchestration_idempotency_key, EvolutionActorProvenance,
    EvolutionIntegrityError, EvolutionTriggerEnvelopeV1, EvolutionTriggerFamily,
    ORCHESTRATION_SCHEMA_VERSION_V1,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthoritativeTriggerSourceV1 {
    pub(crate) workspace_id: String,
    pub(crate) source_id: String,
    pub(crate) source_revision: u64,
    pub(crate) occurred_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeCompletionKindV1 {
    AgentRun,
    Conversation,
    Verification,
    DelegatedUtility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelevantMutationKindV1 {
    Skill,
    Overlay,
    Policy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriggerProjectionError {
    InvalidSource,
    Integrity,
}

pub(crate) struct EvolutionTriggerProjectorV1;

impl EvolutionTriggerProjectorV1 {
    pub(crate) fn startup_recovery(
        source: AuthoritativeTriggerSourceV1,
    ) -> Result<EvolutionTriggerEnvelopeV1, TriggerProjectionError> {
        project(
            EvolutionTriggerFamily::StartupRecovery,
            "application-lifecycle",
            source,
            100,
            "startup-recovery-due",
            EvolutionActorProvenance::Recovery,
        )
    }

    pub(crate) fn periodic_maintenance(
        source: AuthoritativeTriggerSourceV1,
    ) -> Result<EvolutionTriggerEnvelopeV1, TriggerProjectionError> {
        project(
            EvolutionTriggerFamily::PeriodicMaintenance,
            "maintenance-window",
            source,
            20,
            "maintenance-due",
            EvolutionActorProvenance::RuntimeTrigger,
        )
    }

    pub(crate) fn application_idle_transition(
        source: AuthoritativeTriggerSourceV1,
    ) -> Result<EvolutionTriggerEnvelopeV1, TriggerProjectionError> {
        project(
            EvolutionTriggerFamily::ApplicationIdleTransition,
            "runtime-idle",
            source,
            30,
            "application-became-idle",
            EvolutionActorProvenance::RuntimeTrigger,
        )
    }

    pub(crate) fn runtime_completion(
        kind: RuntimeCompletionKindV1,
        source: AuthoritativeTriggerSourceV1,
    ) -> Result<EvolutionTriggerEnvelopeV1, TriggerProjectionError> {
        let (family, source_kind, reason) = match kind {
            RuntimeCompletionKindV1::AgentRun => (
                EvolutionTriggerFamily::AgentRunCompletion,
                "agent-run",
                "agent-run-completed",
            ),
            RuntimeCompletionKindV1::Conversation => (
                EvolutionTriggerFamily::ConversationCompletion,
                "conversation",
                "conversation-completed",
            ),
            RuntimeCompletionKindV1::Verification => (
                EvolutionTriggerFamily::VerificationCompletion,
                "verification",
                "verification-completed",
            ),
            RuntimeCompletionKindV1::DelegatedUtility => (
                EvolutionTriggerFamily::DelegatedUtilityCompletion,
                "delegated-utility",
                "delegated-utility-completed",
            ),
        };
        project(
            family,
            source_kind,
            source,
            50,
            reason,
            EvolutionActorProvenance::RuntimeTrigger,
        )
    }

    pub(crate) fn explicit_feedback_commit(
        source: AuthoritativeTriggerSourceV1,
    ) -> Result<EvolutionTriggerEnvelopeV1, TriggerProjectionError> {
        project(
            EvolutionTriggerFamily::ExplicitFeedbackCommit,
            "feedback-revision",
            source,
            70,
            "feedback-committed",
            EvolutionActorProvenance::InteractiveUser,
        )
    }

    pub(crate) fn relevant_mutation(
        kind: RelevantMutationKindV1,
        source: AuthoritativeTriggerSourceV1,
    ) -> Result<EvolutionTriggerEnvelopeV1, TriggerProjectionError> {
        let (source_kind, reason) = match kind {
            RelevantMutationKindV1::Skill => ("skill-revision", "skill-changed"),
            RelevantMutationKindV1::Overlay => ("overlay-revision", "overlay-changed"),
            RelevantMutationKindV1::Policy => ("policy-revision", "policy-changed"),
        };
        project(
            EvolutionTriggerFamily::RelevantPolicyOrSkillChange,
            source_kind,
            source,
            80,
            reason,
            EvolutionActorProvenance::RuntimeTrigger,
        )
    }

    pub(crate) fn manual_run_request(
        source: AuthoritativeTriggerSourceV1,
    ) -> Result<EvolutionTriggerEnvelopeV1, TriggerProjectionError> {
        project(
            EvolutionTriggerFamily::ManualRunRequest,
            "manual-request",
            source,
            100,
            "manual-run-requested",
            EvolutionActorProvenance::InteractiveUser,
        )
    }
}

#[derive(Serialize)]
struct TriggerIdentity<'a> {
    family: &'a str,
    workspace_id: &'a str,
    source_kind: &'a str,
    source_id: &'a str,
    source_revision: u64,
}

fn project(
    family: EvolutionTriggerFamily,
    source_kind: &'static str,
    source: AuthoritativeTriggerSourceV1,
    priority: u8,
    reason: &'static str,
    actor: EvolutionActorProvenance,
) -> Result<EvolutionTriggerEnvelopeV1, TriggerProjectionError> {
    if !is_safe_identifier(&source.workspace_id, 128)
        || !is_safe_identifier(&source.source_id, 128)
        || source.occurred_at_ms < 0
    {
        return Err(TriggerProjectionError::InvalidSource);
    }
    let identity = TriggerIdentity {
        family: family.as_str(),
        workspace_id: &source.workspace_id,
        source_kind,
        source_id: &source.source_id,
        source_revision: source.source_revision,
    };
    let digest = orchestration_idempotency_key("trigger", "project", &identity)
        .map_err(map_integrity_error)?;
    Ok(EvolutionTriggerEnvelopeV1 {
        schema_version: ORCHESTRATION_SCHEMA_VERSION_V1,
        trigger_id: format!("trigger:{digest}"),
        family,
        workspace_id: source.workspace_id,
        source_kind: source_kind.into(),
        source_id: source.source_id,
        source_revision: source.source_revision,
        occurred_at_ms: source.occurred_at_ms,
        priority,
        safe_reason_codes: vec![reason.into()],
        actor,
    })
}

fn map_integrity_error(_: EvolutionIntegrityError) -> TriggerProjectionError {
    TriggerProjectionError::Integrity
}
