use serde::{Deserialize, Serialize};

use super::{
    EvolutionActorProvenance, EvolutionResourcePressure, EvolutionTriggerFamily,
    ORCHESTRATION_SCHEMA_VERSION_V1,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvolutionTriggerEnvelopeV1 {
    pub(crate) schema_version: u16,
    pub(crate) trigger_id: String,
    pub(crate) family: EvolutionTriggerFamily,
    pub(crate) workspace_id: String,
    pub(crate) source_kind: String,
    pub(crate) source_id: String,
    pub(crate) source_revision: u64,
    pub(crate) occurred_at_ms: i64,
    pub(crate) priority: u8,
    pub(crate) safe_reason_codes: Vec<String>,
    pub(crate) actor: EvolutionActorProvenance,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvolutionTriggerCountersV1 {
    pub(crate) startup_recovery: u64,
    pub(crate) periodic_maintenance: u64,
    pub(crate) application_idle_transition: u64,
    pub(crate) agent_run_completion: u64,
    pub(crate) conversation_completion: u64,
    pub(crate) explicit_feedback_commit: u64,
    pub(crate) verification_completion: u64,
    pub(crate) delegated_utility_completion: u64,
    pub(crate) relevant_policy_or_skill_change: u64,
    pub(crate) manual_run_request: u64,
}

impl EvolutionTriggerCountersV1 {
    pub(crate) fn increment(&mut self, family: EvolutionTriggerFamily) -> Option<()> {
        let counter = match family {
            EvolutionTriggerFamily::StartupRecovery => &mut self.startup_recovery,
            EvolutionTriggerFamily::PeriodicMaintenance => &mut self.periodic_maintenance,
            EvolutionTriggerFamily::ApplicationIdleTransition => {
                &mut self.application_idle_transition
            }
            EvolutionTriggerFamily::AgentRunCompletion => &mut self.agent_run_completion,
            EvolutionTriggerFamily::ConversationCompletion => &mut self.conversation_completion,
            EvolutionTriggerFamily::ExplicitFeedbackCommit => &mut self.explicit_feedback_commit,
            EvolutionTriggerFamily::VerificationCompletion => &mut self.verification_completion,
            EvolutionTriggerFamily::DelegatedUtilityCompletion => {
                &mut self.delegated_utility_completion
            }
            EvolutionTriggerFamily::RelevantPolicyOrSkillChange => {
                &mut self.relevant_policy_or_skill_change
            }
            EvolutionTriggerFamily::ManualRunRequest => &mut self.manual_run_request,
        };
        *counter = counter.checked_add(1)?;
        Some(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvolutionIdleSnapshotV1 {
    pub(crate) schema_version: u16,
    pub(crate) workspace_id: String,
    pub(crate) captured_at_ms: i64,
    pub(crate) last_user_interaction_at_ms: i64,
    pub(crate) active_agent_generations: u16,
    pub(crate) active_cli_processes: u16,
    pub(crate) active_delegated_utilities: u16,
    pub(crate) pending_approvals: u16,
    pub(crate) active_verifications: u16,
    pub(crate) active_skill_writers: u16,
    pub(crate) active_overlay_writers: u16,
    pub(crate) active_curator_writers: u16,
    pub(crate) active_application_sagas: u16,
    pub(crate) shutting_down: bool,
    pub(crate) resource_pressure: EvolutionResourcePressure,
}

impl EvolutionIdleSnapshotV1 {
    pub(crate) fn has_runtime_or_writer_activity(&self) -> bool {
        self.active_agent_generations > 0
            || self.active_cli_processes > 0
            || self.active_delegated_utilities > 0
            || self.pending_approvals > 0
            || self.active_verifications > 0
            || self.active_skill_writers > 0
            || self.active_overlay_writers > 0
            || self.active_curator_writers > 0
            || self.active_application_sagas > 0
            || self.shutting_down
            || self.resource_pressure == EvolutionResourcePressure::Critical
    }
}

pub(crate) fn empty_idle_snapshot(
    workspace_id: String,
    captured_at_ms: i64,
) -> EvolutionIdleSnapshotV1 {
    EvolutionIdleSnapshotV1 {
        schema_version: ORCHESTRATION_SCHEMA_VERSION_V1,
        workspace_id,
        captured_at_ms,
        last_user_interaction_at_ms: captured_at_ms,
        active_agent_generations: 0,
        active_cli_processes: 0,
        active_delegated_utilities: 0,
        pending_approvals: 0,
        active_verifications: 0,
        active_skill_writers: 0,
        active_overlay_writers: 0,
        active_curator_writers: 0,
        active_application_sagas: 0,
        shutting_down: false,
        resource_pressure: EvolutionResourcePressure::Normal,
    }
}
