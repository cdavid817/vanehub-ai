use super::GENERATION_SCHEMA_VERSION_V1;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct FrozenGenerationInputV1 {
    pub(crate) schema_version: u16,
    pub(crate) request_id: String,
    pub(crate) workspace_id: Option<String>,
    pub(crate) seed_id: String,
    pub(crate) seed_revision: String,
    pub(crate) assessment_attempt_id: String,
    pub(crate) assessment_revision: String,
    pub(crate) assessment_route: String,
    pub(crate) target: Option<GenerationTargetWitnessV1>,
    pub(crate) evidence: GenerationEvidenceWitnessV1,
    pub(crate) effective_skill: Option<EffectiveSkillWitnessV1>,
    pub(crate) curator: Option<CuratorGenerationWitnessV1>,
    pub(crate) policy_revision: u64,
    pub(crate) policy_hash: String,
    pub(crate) consent_revision: u64,
    pub(crate) consent_hash: String,
    pub(crate) model_configuration_hash: String,
    pub(crate) dossier_builder_version: String,
    pub(crate) renderer_version: String,
    pub(crate) validator_version: String,
    pub(crate) frozen_at_ms: i64,
}

impl FrozenGenerationInputV1 {
    pub(crate) fn v1(request_id: String) -> Self {
        Self {
            schema_version: GENERATION_SCHEMA_VERSION_V1,
            request_id,
            workspace_id: None,
            seed_id: String::new(),
            seed_revision: String::new(),
            assessment_attempt_id: String::new(),
            assessment_revision: String::new(),
            assessment_route: String::new(),
            target: None,
            evidence: GenerationEvidenceWitnessV1::default(),
            effective_skill: None,
            curator: None,
            policy_revision: 0,
            policy_hash: String::new(),
            consent_revision: 0,
            consent_hash: String::new(),
            model_configuration_hash: String::new(),
            dossier_builder_version: String::new(),
            renderer_version: String::new(),
            validator_version: String::new(),
            frozen_at_ms: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GenerationTargetWitnessV1 {
    pub(crate) skill_id: String,
    pub(crate) skill_type: String,
    pub(crate) effective_revision: String,
    pub(crate) scope: String,
    pub(crate) trust: String,
    pub(crate) pinned: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GenerationEvidenceWitnessV1 {
    pub(crate) lineage_hash: String,
    pub(crate) sanitizer_version: String,
    pub(crate) evidence_ids: Vec<String>,
    pub(crate) source_revision_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EffectiveSkillWitnessV1 {
    pub(crate) base_hash: String,
    pub(crate) effective_hash: String,
    pub(crate) overlay_revision: u64,
    pub(crate) catalog_revision: String,
    pub(crate) scope_witness: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorGenerationWitnessV1 {
    pub(crate) candidate_id: String,
    pub(crate) candidate_revision: u64,
    pub(crate) candidate_hash: String,
    pub(crate) draft_revision: Option<u64>,
}
