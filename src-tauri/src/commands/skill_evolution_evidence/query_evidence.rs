use serde::{Deserialize, Serialize};
use tauri::State;

use crate::contexts::skill_evolution_evidence::api::SkillEvolutionEvidenceApi;
use crate::contexts::skill_evolution_evidence::infrastructure::{
    EvidenceOverview, EvidencePageRequest, EvidenceQueryScope, EvidenceSeedLineage,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvidenceQueryInput {
    workspace: Option<String>,
    skill_id: Option<String>,
    limit: Option<usize>,
    cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceQueryError {
    code: &'static str,
}

#[tauri::command]
pub(crate) fn query_skill_evolution_evidence(
    api: State<'_, SkillEvolutionEvidenceApi>,
    input: EvidenceQueryInput,
) -> Result<EvidenceOverview, EvidenceQueryError> {
    api.query_overview(&EvidencePageRequest {
        scope: scope(&input),
        limit: input.limit.unwrap_or(50),
        cursor: input.cursor,
    })
    .map_err(|_| query_error())
}

#[tauri::command]
pub(crate) fn get_skill_evolution_seed_lineage(
    api: State<'_, SkillEvolutionEvidenceApi>,
    seed_id: String,
    input: EvidenceQueryInput,
) -> Result<Option<EvidenceSeedLineage>, EvidenceQueryError> {
    api.query_seed_lineage(&seed_id, &scope(&input))
        .map_err(|_| query_error())
}

fn scope(input: &EvidenceQueryInput) -> EvidenceQueryScope {
    EvidenceQueryScope {
        workspace: input.workspace.clone(),
        skill_id: input.skill_id.clone(),
    }
}

fn query_error() -> EvidenceQueryError {
    EvidenceQueryError {
        code: "evidence-query-failed",
    }
}
