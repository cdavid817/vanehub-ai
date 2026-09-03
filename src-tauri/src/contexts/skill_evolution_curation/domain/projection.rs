use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorCandidateSnapshot {
    pub(crate) schema_version: u16,
    pub(crate) candidate_id: String,
    pub(crate) workspace_id: String,
    pub(crate) seed_id: String,
    pub(crate) seed_revision: String,
    pub(crate) assessment_attempt_id: String,
    pub(crate) assessment_revision: String,
    pub(crate) target_skill_id: String,
    pub(crate) target_revision: String,
    pub(crate) overlay_scope: String,
    pub(crate) route: CuratorRoute,
    pub(crate) risk: CuratorRisk,
    pub(crate) confidence: CuratorConfidence,
    pub(crate) evidence_ids: Vec<String>,
    pub(crate) evidence_sources: Vec<CuratorEvidenceSource>,
    pub(crate) quality_checks: Vec<CuratorQualityCheck>,
    pub(crate) assessment_witness_hash: String,
    pub(crate) policy_witness_hash: String,
    pub(crate) witness_hash: String,
    pub(crate) state: CuratorCandidateState,
    pub(crate) staleness: Vec<CuratorStalenessReason>,
    pub(crate) revision: u64,
    pub(crate) created_at_ms: i64,
    pub(crate) updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorEvidenceSource {
    pub(crate) evidence_id: String,
    pub(crate) evidence_revision: String,
    pub(crate) lineage_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AssessmentCompletionEnvelopeV1 {
    pub(crate) schema_version: u16,
    pub(crate) assessment_attempt_id: String,
    pub(crate) assessment_revision: String,
    pub(crate) current: bool,
    pub(crate) route: CuratorAssessmentRoute,
    pub(crate) witness_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CuratorIntakeOutcome {
    CandidateCreated { candidate_id: String },
    ExistingCandidate { candidate_id: String },
    NonApprovableRecorded,
    NonCurrentRejected,
    PurgedEvidenceRejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorQualityCheck {
    pub(crate) code: String,
    pub(crate) result: CuratorCheckResult,
    pub(crate) reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorDraftRevision {
    pub(crate) draft_id: String,
    pub(crate) candidate_id: String,
    pub(crate) revision: u64,
    pub(crate) kind: CuratorDraftKind,
    pub(crate) target_skill_id: String,
    pub(crate) target_revision: String,
    pub(crate) overlay_scope: String,
    pub(crate) body_hash: String,
    pub(crate) evidence_ids: Vec<String>,
    pub(crate) rationale: String,
    pub(crate) expected_effective_change: String,
    pub(crate) base_hash: String,
    pub(crate) base_package_hash: String,
    pub(crate) effective_hash: String,
    pub(crate) overlay_revision: Option<u64>,
    pub(crate) pin_witness: String,
    pub(crate) trust_witness: String,
    pub(crate) conflict_witness: String,
    pub(crate) created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorDraftAssessment {
    pub(crate) assessment_id: String,
    pub(crate) candidate_id: String,
    pub(crate) candidate_revision: u64,
    pub(crate) draft_id: String,
    pub(crate) draft_revision: u64,
    pub(crate) draft_hash: String,
    pub(crate) candidate_witness_hash: String,
    pub(crate) target_skill_id: String,
    pub(crate) target_revision: String,
    pub(crate) checks: Vec<CuratorQualityCheck>,
    pub(crate) approvable: bool,
    pub(crate) model_evaluation_allowed: bool,
    pub(crate) model_consulted: bool,
    pub(crate) model_fallback_reason: Option<String>,
    pub(crate) witness_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorDecision {
    pub(crate) decision_id: String,
    pub(crate) candidate_id: String,
    pub(crate) candidate_revision: u64,
    pub(crate) kind: CuratorDecisionKind,
    pub(crate) actor_class: CuratorActorClass,
    pub(crate) reason_code: String,
    pub(crate) note_hash: Option<String>,
    pub(crate) preview_hash: Option<String>,
    pub(crate) decided_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorAuditEvent {
    pub(crate) candidate_id: String,
    pub(crate) sequence: u64,
    pub(crate) event_kind: CuratorEventKind,
    pub(crate) actor_class: CuratorActorClass,
    pub(crate) occurred_at_ms: i64,
    pub(crate) prior_state: Option<CuratorCandidateState>,
    pub(crate) next_state: CuratorCandidateState,
    pub(crate) object_revision: u64,
    pub(crate) reason_code: Option<String>,
    pub(crate) prior_hash: Option<String>,
    pub(crate) event_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorApplication {
    pub(crate) application_id: String,
    pub(crate) candidate_id: String,
    pub(crate) decision_id: String,
    pub(crate) status: CuratorApplicationStatus,
    pub(crate) approved_witness_hash: String,
    pub(crate) overlay_revision: Option<String>,
    pub(crate) overlay_history_id: Option<String>,
    pub(crate) failure_code: Option<String>,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorOutboxRecord {
    pub(crate) outbox_id: String,
    pub(crate) application_id: String,
    pub(crate) candidate_id: String,
    pub(crate) witness_hash: String,
    pub(crate) attempt_count: u32,
    pub(crate) available_at_ms: i64,
    pub(crate) completed_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorCandidateSummary {
    pub(crate) candidate_id: String,
    pub(crate) target_skill_id: String,
    pub(crate) state: CuratorCandidateState,
    pub(crate) route: CuratorRoute,
    pub(crate) risk: CuratorRisk,
    pub(crate) draft_ready: bool,
    pub(crate) staleness: Vec<CuratorStalenessReason>,
    pub(crate) revision: u64,
    pub(crate) updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorQueuePage {
    pub(crate) items: Vec<CuratorCandidateSummary>,
    pub(crate) next_cursor: Option<String>,
    pub(crate) total_count: u64,
    pub(crate) complete: bool,
}
