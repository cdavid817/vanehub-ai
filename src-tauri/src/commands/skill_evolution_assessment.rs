use crate::contexts::skill_evolution_assessment::api::{
    AssessmentApiError, SkillEvolutionAssessmentApi,
};
use crate::contexts::skill_evolution_assessment::domain::ModelEvaluationConsent;
use serde::Deserialize;
use serde_json::Value;
use tauri::State;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AssessmentQueryInput {
    workspace: Option<String>,
    skill_id: Option<String>,
    seed_id: Option<String>,
    include_history: Option<bool>,
    limit: Option<usize>,
    cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UpdateConsentInput {
    enabled: bool,
    evaluator_policy_version: String,
    disclosure_version: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReassessmentInput {
    seed_id: String,
    expected_witness_hash: Option<String>,
}

#[tauri::command]
pub(crate) fn query_skill_evolution_assessments(
    api: State<'_, SkillEvolutionAssessmentApi>,
    input: AssessmentQueryInput,
) -> Result<Value, String> {
    api.query(
        input.workspace.as_deref(),
        input.skill_id.as_deref(),
        input.seed_id.as_deref(),
        input.include_history.unwrap_or(false),
        input.limit.unwrap_or(20),
        input.cursor.as_deref(),
    )
    .map_err(command_error)
}

#[tauri::command]
pub(crate) fn get_skill_evolution_assessment(
    api: State<'_, SkillEvolutionAssessmentApi>,
    attempt_id: String,
) -> Result<Option<Value>, String> {
    api.detail(&attempt_id).map_err(command_error)
}

#[tauri::command]
pub(crate) fn get_skill_evolution_assessment_policy(
    api: State<'_, SkillEvolutionAssessmentApi>,
) -> Result<Value, String> {
    api.policy().map_err(command_error)
}

#[tauri::command]
pub(crate) fn update_skill_evolution_assessment_consent(
    api: State<'_, SkillEvolutionAssessmentApi>,
    input: UpdateConsentInput,
) -> Result<Value, String> {
    let now_ms = chrono::Utc::now().timestamp_millis();
    api.update_consent(ModelEvaluationConsent {
        policy_version: input.evaluator_policy_version,
        disclosure_version: input.disclosure_version,
        enabled: input.enabled,
        changed_at_ms: now_ms,
        local_actor: "local-ui".to_string(),
    })
    .map_err(command_error)
}

#[tauri::command]
pub(crate) fn schedule_skill_evolution_reassessment(
    api: State<'_, SkillEvolutionAssessmentApi>,
    input: ReassessmentInput,
) -> Result<Value, String> {
    api.schedule(
        &input.seed_id,
        input.expected_witness_hash.as_deref(),
        chrono::Utc::now().timestamp_millis(),
    )
    .map_err(command_error)
}

fn command_error(error: AssessmentApiError) -> String {
    error.code().to_string()
}
