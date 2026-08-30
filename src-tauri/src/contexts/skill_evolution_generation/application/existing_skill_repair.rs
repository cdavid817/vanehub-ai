use std::collections::BTreeSet;

use crate::contexts::skill_evolution_generation::domain::{
    GenerationValidationStatus, RenderedGenerationArtifactV1, StructuredDraftV1,
};

use super::{
    render_generation_artifact, validate_existing_skill_draft, ExistingSkillValidationPort,
    ExistingSkillValidationRequestV1, ExistingSkillValidationResultV1, GenerationRenderRequestV1,
};

const REPAIRABLE_REASON_CODES_V1: [&str; 9] = [
    "generation_structure_invalid",
    "generation_budget_invalid",
    "generation_exact_anchor_invalid",
    "generation_verification_incomplete",
    "generation_quality_contract_invalid",
    "generation_model_judge_rejected",
    "generation_preview_invalid",
    "vague_or_untestable_guidance",
    "material_contradiction",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExistingSkillRepairRequestV1 {
    pub(crate) prior_structured_draft: StructuredDraftV1,
    pub(crate) safe_reason_codes: Vec<String>,
}

pub(crate) trait ExistingSkillRepairPort {
    fn repair(
        &self,
        request: &ExistingSkillRepairRequestV1,
    ) -> Result<StructuredDraftV1, &'static str>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExistingSkillValidationWithRepairV1 {
    pub(crate) attempts: Vec<ExistingSkillValidationResultV1>,
    pub(crate) final_draft: StructuredDraftV1,
    pub(crate) final_artifact: RenderedGenerationArtifactV1,
    pub(crate) repair_performed: bool,
    pub(crate) repair_failure_code: Option<String>,
}

pub(crate) fn validate_existing_skill_with_one_repair(
    validation_port: &dyn ExistingSkillValidationPort,
    repair_port: &dyn ExistingSkillRepairPort,
    mut request: ExistingSkillValidationRequestV1,
    repaired_artifact_id: &str,
    repair_already_used: bool,
) -> ExistingSkillValidationWithRepairV1 {
    let first = validate_existing_skill_draft(validation_port, &request);
    let mut outcome = ExistingSkillValidationWithRepairV1 {
        attempts: vec![first.clone()],
        final_draft: request.draft.clone(),
        final_artifact: request.artifact.clone(),
        repair_performed: false,
        repair_failure_code: None,
    };
    if first.status == GenerationValidationStatus::Passed
        || repair_already_used
        || !repairable(&first.safe_reason_codes)
    {
        return outcome;
    }
    let repair_request = ExistingSkillRepairRequestV1 {
        prior_structured_draft: request.draft.clone(),
        safe_reason_codes: first.safe_reason_codes,
    };
    let repaired_draft = match repair_port.repair(&repair_request) {
        Ok(draft) => draft,
        Err(code) => {
            outcome.repair_failure_code = Some(code.into());
            return outcome;
        }
    };
    let artifact = match render_generation_artifact(&GenerationRenderRequestV1 {
        artifact_id: repaired_artifact_id,
        expected_kind: request.plan.artifact_kind,
        draft: &repaired_draft,
        allowed_built_in_tools: &BTreeSet::new(),
    }) {
        Ok(artifact) => artifact,
        Err(_) => {
            outcome.repair_failure_code = Some("generation_repair_render_invalid".into());
            return outcome;
        }
    };
    request.validation_id = format!("{}-repair-1", request.validation_id);
    request.draft = repaired_draft.clone();
    request.artifact = artifact.clone();
    let second = validate_existing_skill_draft(validation_port, &request);
    outcome.attempts.push(second);
    outcome.final_draft = repaired_draft;
    outcome.final_artifact = artifact;
    outcome.repair_performed = true;
    outcome
}

fn repairable(reason_codes: &[String]) -> bool {
    !reason_codes.is_empty()
        && reason_codes.iter().all(|code| {
            REPAIRABLE_REASON_CODES_V1
                .iter()
                .any(|allowed| code == allowed)
        })
}
