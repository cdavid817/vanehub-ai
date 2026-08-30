use super::{CuratorCandidateState, CuratorQualityCheck};

pub(crate) const CURATOR_DRAFT_CHECK_ORDER_V1: [&str; 9] = [
    "privacy_residue",
    "evidence_sufficiency",
    "duplicate_knowledge",
    "transient_incident",
    "guidance_specificity",
    "evidence_consistency",
    "target_compatibility",
    "executable_content_risk",
    "target_lifecycle_mutability",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorDraftLessonShape {
    pub(crate) trigger: String,
    pub(crate) required_behavior: String,
    pub(crate) prohibited_behavior: String,
    pub(crate) verification: String,
    pub(crate) environment: String,
    pub(crate) content_kinds: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorDraftReviewBinding {
    pub(crate) candidate_id: String,
    pub(crate) candidate_revision: u64,
    pub(crate) candidate_witness_hash: String,
    pub(crate) state: CuratorCandidateState,
    pub(crate) assessment_attempt_id: String,
    pub(crate) assessment_revision: String,
    pub(crate) target_skill_id: String,
    pub(crate) target_revision: String,
    pub(crate) draft_id: String,
    pub(crate) draft_revision: u64,
    pub(crate) draft_hash: String,
    pub(crate) draft_kind: String,
    pub(crate) rationale: String,
    pub(crate) expected_effective_change: String,
    pub(crate) evidence_ids: Vec<String>,
    pub(crate) original_checks: Vec<CuratorQualityCheck>,
    pub(crate) original_lesson_shape: CuratorDraftLessonShape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorDraftQualityInput {
    pub(crate) candidate_id: String,
    pub(crate) candidate_revision: u64,
    pub(crate) candidate_witness_hash: String,
    pub(crate) assessment_attempt_id: String,
    pub(crate) assessment_revision: String,
    pub(crate) target_skill_id: String,
    pub(crate) target_revision: String,
    pub(crate) draft_id: String,
    pub(crate) draft_revision: u64,
    pub(crate) draft_hash: String,
    pub(crate) evidence_ids: Vec<String>,
    pub(crate) original_checks: Vec<CuratorQualityCheck>,
    pub(crate) original_lesson_shape: CuratorDraftLessonShape,
    pub(crate) lesson_shape: CuratorDraftLessonShape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorDraftQualityReceipt {
    pub(crate) candidate_witness_hash: String,
    pub(crate) target_skill_id: String,
    pub(crate) target_revision: String,
    pub(crate) draft_hash: String,
    pub(crate) checks: Vec<CuratorQualityCheck>,
    pub(crate) deterministic_approvable: bool,
    pub(crate) model_evaluation_allowed: bool,
    pub(crate) model_consulted: bool,
    pub(crate) model_fallback_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CuratorDraftReviewRequest<'a> {
    pub(crate) candidate_id: &'a str,
    pub(crate) expected_candidate_revision: u64,
    pub(crate) expected_draft_revision: u64,
}
