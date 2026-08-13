use serde::{Deserialize, Serialize};
use tauri::State;

use crate::contexts::skill_evolution_evidence::api::SkillEvolutionEvidenceApi;
use crate::contexts::skill_evolution_evidence::infrastructure::EvidencePurgeOutcome;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PurgeEvidenceInput {
    operation_id: String,
    workspace: Option<String>,
    skill_id: String,
    confirmed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PurgeEvidenceError {
    code: &'static str,
}

#[tauri::command]
pub(crate) fn purge_skill_evolution_evidence(
    api: State<'_, SkillEvolutionEvidenceApi>,
    input: PurgeEvidenceInput,
) -> Result<EvidencePurgeOutcome, PurgeEvidenceError> {
    api.purge_skill_scope(
        input.operation_id,
        input.workspace,
        input.skill_id,
        input.confirmed,
    )
    .map_err(|error| PurgeEvidenceError { code: error.code() })
}
