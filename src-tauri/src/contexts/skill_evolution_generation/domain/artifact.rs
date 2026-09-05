use super::{
    GeneratedArtifactKind, GenerationHandoffStatus, GenerationQuarantineStatus,
    GenerationValidationStatus,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MutationPlanV1 {
    pub(crate) schema_version: u16,
    pub(crate) plan_id: String,
    pub(crate) artifact_kind: GeneratedArtifactKind,
    pub(crate) target: MutationTargetV1,
    pub(crate) rationale: String,
    pub(crate) lesson: GeneratedLessonShapeV1,
    pub(crate) evidence_citations: Vec<GenerationCitationV1>,
    pub(crate) expected_behavior: String,
    pub(crate) verification_steps: Vec<GeneratedVerificationStepV1>,
    pub(crate) content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum MutationTargetV1 {
    ExistingSkill {
        skill_id: String,
        effective_revision: String,
        overlay_scope: String,
    },
    NewSkill {
        candidate_id: String,
        scope: String,
        workspace_id: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneratedLessonShapeV1 {
    pub(crate) trigger: String,
    pub(crate) action: String,
    pub(crate) verification: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GenerationCitationV1 {
    pub(crate) claim_id: String,
    pub(crate) dossier_section: String,
    pub(crate) source_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneratedVerificationStepV1 {
    pub(crate) step_id: String,
    pub(crate) action_code: String,
    pub(crate) expected_code: String,
    pub(crate) citation_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum StructuredDraftV1 {
    OverlayLearnBlock {
        guidance: String,
    },
    OverlayExactPatch {
        old_string: String,
        new_string: String,
        replace_all: bool,
    },
    NewSkill {
        candidate_id: String,
        name: String,
        description: String,
        skill_type: String,
        version: String,
        built_in_tools: Vec<String>,
        instructions: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RenderedGenerationArtifactV1 {
    pub(crate) artifact_id: String,
    pub(crate) artifact_kind: GeneratedArtifactKind,
    pub(crate) renderer_version: String,
    pub(crate) media_type: String,
    pub(crate) content: String,
    pub(crate) size_bytes: u32,
    pub(crate) content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GenerationValidationV1 {
    pub(crate) validation_id: String,
    pub(crate) artifact_id: String,
    pub(crate) validator_version: String,
    pub(crate) status: GenerationValidationStatus,
    pub(crate) checks: Vec<GenerationValidationCheckV1>,
    pub(crate) preview_witness_hash: Option<String>,
    pub(crate) report_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GenerationValidationCheckV1 {
    pub(crate) code: String,
    pub(crate) status: GenerationValidationStatus,
    pub(crate) reason_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GenerationExportV1 {
    pub(crate) export_id: String,
    pub(crate) dossier_id: String,
    pub(crate) format: String,
    pub(crate) schema_version: u16,
    pub(crate) completeness: bool,
    pub(crate) redaction_manifest_hash: String,
    pub(crate) content_hash: String,
    pub(crate) size_bytes: u64,
    pub(crate) created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct QuarantinedSkillProposalV1 {
    pub(crate) proposal_id: String,
    pub(crate) job_id: String,
    pub(crate) status: GenerationQuarantineStatus,
    pub(crate) candidate_id: String,
    pub(crate) scope: String,
    pub(crate) workspace_id: Option<String>,
    pub(crate) artifact_hash: String,
    pub(crate) catalog_witness_hash: String,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneratedReviewPackageV1 {
    pub(crate) package_id: String,
    pub(crate) job_id: String,
    pub(crate) attempt_id: String,
    pub(crate) dossier_id: String,
    pub(crate) plan_hash: String,
    pub(crate) artifact_id: String,
    pub(crate) validation_id: String,
    pub(crate) preview_witness_hash: Option<String>,
    pub(crate) policy_hash: String,
    pub(crate) consent_hash: String,
    pub(crate) model_provenance_hash: String,
    pub(crate) permanently_manual: bool,
    pub(crate) handoff_status: GenerationHandoffStatus,
    pub(crate) curator_candidate_id: Option<String>,
    pub(crate) created_at_ms: i64,
}
