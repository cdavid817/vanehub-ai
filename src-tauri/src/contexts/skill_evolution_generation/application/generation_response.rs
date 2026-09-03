use serde::Deserialize;

use crate::contexts::skill_evolution_generation::domain::{MutationPlanV1, StructuredDraftV1};

use super::{validate_mutation_plan_structure, GenerationModelError, GenerationModelStage};

const MAX_RESPONSE_BYTES_V1: usize = 64 * 1024;
const MAX_DRAFT_FIELD_BYTES_V1: usize = 32 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParsedGenerationResponseV1 {
    MutationPlan(MutationPlanV1),
    StructuredDraft(StructuredDraftV1),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StrictEnvelope<T> {
    schema_version: u16,
    result: T,
}

pub(crate) fn parse_generation_response(
    stage: GenerationModelStage,
    response_json: &str,
) -> Result<ParsedGenerationResponseV1, GenerationModelError> {
    if response_json.is_empty() || response_json.len() > MAX_RESPONSE_BYTES_V1 {
        return Err(GenerationModelError::InvalidRequest);
    }
    match stage {
        GenerationModelStage::PlanMutation => parse_plan(response_json),
        GenerationModelStage::SynthesizeStructuredDraft
        | GenerationModelStage::RepairStructuredDraft => parse_draft(response_json),
    }
}

fn parse_plan(response_json: &str) -> Result<ParsedGenerationResponseV1, GenerationModelError> {
    let envelope: StrictEnvelope<MutationPlanV1> =
        serde_json::from_str(response_json).map_err(|_| GenerationModelError::InvalidRequest)?;
    if envelope.schema_version != 1 || validate_mutation_plan_structure(&envelope.result).is_err() {
        return Err(GenerationModelError::InvalidRequest);
    }
    Ok(ParsedGenerationResponseV1::MutationPlan(envelope.result))
}

fn parse_draft(response_json: &str) -> Result<ParsedGenerationResponseV1, GenerationModelError> {
    let envelope: StrictEnvelope<StructuredDraftV1> =
        serde_json::from_str(response_json).map_err(|_| GenerationModelError::InvalidRequest)?;
    if envelope.schema_version != 1 || draft_size(&envelope.result) > MAX_DRAFT_FIELD_BYTES_V1 {
        return Err(GenerationModelError::InvalidRequest);
    }
    Ok(ParsedGenerationResponseV1::StructuredDraft(envelope.result))
}

fn draft_size(draft: &StructuredDraftV1) -> usize {
    match draft {
        StructuredDraftV1::OverlayLearnBlock { guidance } => guidance.len(),
        StructuredDraftV1::OverlayExactPatch {
            old_string,
            new_string,
            ..
        } => old_string.len() + new_string.len(),
        StructuredDraftV1::NewSkill {
            candidate_id,
            name,
            description,
            skill_type,
            version,
            built_in_tools,
            instructions,
        } => {
            candidate_id.len()
                + name.len()
                + description.len()
                + skill_type.len()
                + version.len()
                + built_in_tools.iter().map(String::len).sum::<usize>()
                + instructions.len()
        }
    }
}
