use std::collections::BTreeSet;

use crate::contexts::skill_evolution_generation::domain::{
    GenerationValidationCheckV1, GenerationValidationStatus, MutationPlanV1,
    RenderedGenerationArtifactV1, StructuredDraftV1,
};

pub(crate) const GENERATION_DRAFT_CHECK_ORDER_V1: [&str; 9] = [
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
pub(crate) struct ExistingSkillValidationRequestV1 {
    pub(crate) validation_id: String,
    pub(crate) plan: MutationPlanV1,
    pub(crate) draft: StructuredDraftV1,
    pub(crate) artifact: RenderedGenerationArtifactV1,
    pub(crate) frozen_skill_id: String,
    pub(crate) frozen_revision: String,
    pub(crate) overlay_scope: String,
    pub(crate) frozen_effective_content: String,
    pub(crate) frozen_overlay_witness_hash: String,
    pub(crate) current_overlay_witness_hash: String,
    pub(crate) registered_citations: BTreeSet<(String, String)>,
    pub(crate) estimated_tokens: u32,
    pub(crate) maximum_tokens: u32,
    pub(crate) pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenerationSafetyReceiptV1 {
    pub(crate) sanitizer_version: String,
    pub(crate) content_hash: String,
    pub(crate) privacy_passed: bool,
    pub(crate) injection_passed: bool,
    pub(crate) prohibited_content_passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenerationQualityReceiptV1 {
    pub(crate) artifact_hash: String,
    pub(crate) checks: Vec<GenerationValidationCheckV1>,
    pub(crate) stricter_judge_passed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenerationOverlayPreviewReceiptV1 {
    pub(crate) artifact_hash: String,
    pub(crate) target_revision: String,
    pub(crate) overlay_witness_hash: String,
    pub(crate) exact_anchor_matches: u16,
    pub(crate) unrelated_deletion: bool,
    pub(crate) can_commit: bool,
    pub(crate) preview_witness_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExistingSkillValidationResultV1 {
    pub(crate) status: GenerationValidationStatus,
    pub(crate) checks: Vec<GenerationValidationCheckV1>,
    pub(crate) preview_witness_hash: Option<String>,
    pub(crate) report_hash: String,
    pub(crate) safe_reason_codes: Vec<String>,
}

pub(crate) trait ExistingSkillValidationPort {
    fn scan(
        &self,
        artifact: &RenderedGenerationArtifactV1,
    ) -> Result<GenerationSafetyReceiptV1, &'static str>;

    fn is_duplicate(&self, artifact_hash: &str) -> Result<bool, &'static str>;

    fn quality(
        &self,
        request: &ExistingSkillValidationRequestV1,
    ) -> Result<GenerationQualityReceiptV1, &'static str>;

    fn preview(
        &self,
        request: &ExistingSkillValidationRequestV1,
    ) -> Result<GenerationOverlayPreviewReceiptV1, &'static str>;
}
