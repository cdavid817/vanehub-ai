use serde::{Deserialize, Serialize};

use super::{
    AutoEligibilityResult, CircuitBreakerStatus, EvolutionActorProvenance, ProbationStatus,
    RateReservationStatus,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DeterministicCorrectionDraftV1 {
    pub(crate) draft_id: String,
    pub(crate) workspace_id: String,
    pub(crate) target_skill_id: String,
    pub(crate) authorization_id: String,
    pub(crate) assessment_id: String,
    pub(crate) producer_version: String,
    pub(crate) content_hash: String,
    pub(crate) content_size_bytes: u16,
    pub(crate) provenance: String,
    pub(crate) source_witness_hash: String,
    pub(crate) created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EligibilityPredicateV1 {
    pub(crate) condition: String,
    pub(crate) passed: bool,
    pub(crate) safe_reason_code: Option<String>,
    pub(crate) witness_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AutoApplyEligibilityV1 {
    pub(crate) eligibility_id: String,
    pub(crate) run_id: String,
    pub(crate) draft_id: String,
    pub(crate) target_skill_id: String,
    pub(crate) result: AutoEligibilityResult,
    pub(crate) predicates: Vec<EligibilityPredicateV1>,
    pub(crate) proof_hash: String,
    pub(crate) overlay_preview_hash: Option<String>,
    pub(crate) evaluated_at_ms: i64,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AutoRateReservationV1 {
    pub(crate) reservation_id: String,
    pub(crate) run_id: String,
    pub(crate) workspace_id: String,
    pub(crate) skill_id: String,
    pub(crate) status: RateReservationStatus,
    pub(crate) application_id: Option<String>,
    pub(crate) reserved_at_ms: i64,
    pub(crate) updated_at_ms: i64,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AutoApplyCircuitBreakerV1 {
    pub(crate) breaker_id: String,
    pub(crate) workspace_id: String,
    pub(crate) skill_id: Option<String>,
    pub(crate) status: CircuitBreakerStatus,
    pub(crate) safe_cause_code: Option<String>,
    pub(crate) source_run_id: Option<String>,
    pub(crate) source_application_id: Option<String>,
    pub(crate) health_check_version: String,
    pub(crate) health_probe_passed: bool,
    pub(crate) acknowledged_by: Option<EvolutionActorProvenance>,
    pub(crate) opened_at_ms: Option<i64>,
    pub(crate) updated_at_ms: i64,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AutomaticEvolutionApplicationV1 {
    pub(crate) application_id: String,
    pub(crate) run_id: String,
    pub(crate) eligibility_id: String,
    pub(crate) preflight_witness_hash: String,
    pub(crate) policy_witness_hash: String,
    pub(crate) rate_reservation_id: String,
    pub(crate) curator_application_id: String,
    pub(crate) overlay_application_id: String,
    pub(crate) target_skill_id: String,
    pub(crate) prior_effective_hash: String,
    pub(crate) resulting_effective_hash: String,
    pub(crate) actor: EvolutionActorProvenance,
    pub(crate) committed_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AutoApplyProbationV1 {
    pub(crate) probation_id: String,
    pub(crate) application_id: String,
    pub(crate) workspace_id: String,
    pub(crate) skill_id: String,
    pub(crate) status: ProbationStatus,
    pub(crate) prior_effective_hash: String,
    pub(crate) current_effective_hash: String,
    pub(crate) evidence_fingerprint: String,
    pub(crate) evidence_categories: Vec<String>,
    pub(crate) baseline_witness_hash: String,
    pub(crate) starts_at_ms: i64,
    pub(crate) ends_at_ms: i64,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProbationObservationV1 {
    pub(crate) observation_id: String,
    pub(crate) probation_id: String,
    pub(crate) source_kind: String,
    pub(crate) source_id: String,
    pub(crate) source_revision: u64,
    pub(crate) verified: bool,
    pub(crate) negative: bool,
    pub(crate) baseline_exceeded: bool,
    pub(crate) harmful_correction: bool,
    pub(crate) safe_category: String,
    pub(crate) witness_hash: String,
    pub(crate) observed_at_ms: i64,
}
