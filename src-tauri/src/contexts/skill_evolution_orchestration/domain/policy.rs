use serde::{Deserialize, Serialize};

use super::{EvolutionActorProvenance, EvolutionPolicyMode, EvolutionRunBudgetV1};

pub(crate) const ORCHESTRATION_DISCLOSURE_VERSION_V1: &str =
    "skill-evolution-orchestration-disclosure-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvolutionConsentWitnessV1 {
    pub(crate) disclosure_version: String,
    pub(crate) acknowledged_policy_revision: u64,
    pub(crate) actor: EvolutionActorProvenance,
    pub(crate) acknowledged_at_ms: i64,
    pub(crate) revoked_at_ms: Option<i64>,
    pub(crate) witness_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvolutionOrchestrationPolicyV1 {
    pub(crate) schema_version: u16,
    pub(crate) workspace_id: String,
    pub(crate) mode: EvolutionPolicyMode,
    pub(crate) allowed_skill_ids: Vec<String>,
    pub(crate) consent: Option<EvolutionConsentWitnessV1>,
    pub(crate) automatic_budget: EvolutionRunBudgetV1,
    pub(crate) manual_budget: EvolutionRunBudgetV1,
    pub(crate) user_idle_ms: u64,
    pub(crate) maximum_idle_wait_ms: u64,
    pub(crate) notify_routine_completion: bool,
    pub(crate) revision: u64,
    pub(crate) created_at_ms: i64,
    pub(crate) updated_at_ms: i64,
}

impl EvolutionOrchestrationPolicyV1 {
    pub(crate) fn default_off(workspace_id: String, now_ms: i64) -> Self {
        Self {
            schema_version: 1,
            workspace_id,
            mode: EvolutionPolicyMode::Off,
            allowed_skill_ids: Vec::new(),
            consent: None,
            automatic_budget: EvolutionRunBudgetV1::automatic_v1(),
            manual_budget: EvolutionRunBudgetV1::manual_v1(),
            user_idle_ms: 60_000,
            maximum_idle_wait_ms: 900_000,
            notify_routine_completion: false,
            revision: 0,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReusableCorrectionAuthorizationV1 {
    pub(crate) authorization_id: String,
    pub(crate) feedback_id: String,
    pub(crate) feedback_revision: u64,
    pub(crate) disclosure_version: String,
    pub(crate) authorized: bool,
    pub(crate) actor: EvolutionActorProvenance,
    pub(crate) witness_hash: String,
    pub(crate) created_at_ms: i64,
    pub(crate) revoked_at_ms: Option<i64>,
}
