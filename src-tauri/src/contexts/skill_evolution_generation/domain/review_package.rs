use super::{
    GeneratedReviewPackageV1, GenerationModelCallRecordV1, GenerationValidationV1, MutationPlanV1,
    QuarantinedSkillProposalV1, RenderedGenerationArtifactV1, StructuredDraftV1,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneratedReviewPackagePayloadV1 {
    pub(crate) package: GeneratedReviewPackageV1,
    pub(crate) dossier_revision: u64,
    pub(crate) dossier_hash: String,
    pub(crate) plan: MutationPlanV1,
    pub(crate) structured_draft: StructuredDraftV1,
    pub(crate) rendered_artifact: RenderedGenerationArtifactV1,
    pub(crate) validation: GenerationValidationV1,
    pub(crate) model_calls: Vec<GenerationModelCallRecordV1>,
    pub(crate) quarantine: Option<QuarantinedSkillProposalV1>,
    pub(crate) parent_package_id: Option<String>,
    pub(crate) user_edited: bool,
    pub(crate) auto_apply_excluded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PreparedGenerationReviewPackageV1 {
    pub(crate) payload: GeneratedReviewPackagePayloadV1,
    pub(crate) package_hash: String,
}
