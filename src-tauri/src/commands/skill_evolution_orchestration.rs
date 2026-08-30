use crate::contexts::skill_evolution_orchestration::api::SkillEvolutionOrchestrationApi;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvolutionCommandError {
    code: String,
}

type EvolutionCommandResult = Result<Value, EvolutionCommandError>;

fn boundary(result: Result<Value, String>) -> EvolutionCommandResult {
    result.map_err(|code| EvolutionCommandError { code })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvolutionQueryInput {
    workspace_id: String,
    cursor: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvolutionPolicyUpdateInput {
    workspace_id: String,
    expected_revision: u64,
    mode: String,
    allowed_skill_ids: Vec<String>,
    acknowledge_current_disclosure: bool,
}

#[tauri::command]
pub(crate) fn get_skill_evolution_scheduler_overview(
    api: State<'_, SkillEvolutionOrchestrationApi>,
    workspace_id: String,
) -> EvolutionCommandResult {
    boundary(api.scheduler_overview(&workspace_id))
}

#[tauri::command]
pub(crate) fn get_skill_evolution_policy(
    api: State<'_, SkillEvolutionOrchestrationApi>,
    workspace_id: String,
) -> EvolutionCommandResult {
    boundary(api.policy_projection(&workspace_id, now()))
}

#[tauri::command]
pub(crate) fn update_skill_evolution_policy(
    api: State<'_, SkillEvolutionOrchestrationApi>,
    input: EvolutionPolicyUpdateInput,
) -> EvolutionCommandResult {
    boundary(api.update_policy_projection(
        &input.workspace_id,
        input.expected_revision,
        &input.mode,
        input.allowed_skill_ids,
        input.acknowledge_current_disclosure,
        now(),
    ))
}

#[tauri::command]
pub(crate) fn list_skill_evolution_runs(
    api: State<'_, SkillEvolutionOrchestrationApi>,
    input: EvolutionQueryInput,
) -> EvolutionCommandResult {
    boundary(api.runs(
        &input.workspace_id,
        input.cursor.as_deref(),
        input.limit.unwrap_or(50),
    ))
}

#[tauri::command]
pub(crate) fn get_skill_evolution_run(
    api: State<'_, SkillEvolutionOrchestrationApi>,
    run_id: String,
) -> EvolutionCommandResult {
    boundary(api.run_detail(&run_id))
}

#[tauri::command]
pub(crate) fn list_skill_evolution_eligibility(
    api: State<'_, SkillEvolutionOrchestrationApi>,
    input: EvolutionQueryInput,
) -> EvolutionCommandResult {
    boundary(api.eligibility(
        &input.workspace_id,
        input.cursor.as_deref(),
        input.limit.unwrap_or(50),
    ))
}

#[tauri::command]
pub(crate) fn list_skill_evolution_applications(
    api: State<'_, SkillEvolutionOrchestrationApi>,
    input: EvolutionQueryInput,
) -> EvolutionCommandResult {
    boundary(api.applications(
        &input.workspace_id,
        input.cursor.as_deref(),
        input.limit.unwrap_or(50),
    ))
}

#[tauri::command]
pub(crate) fn list_skill_evolution_probations(
    api: State<'_, SkillEvolutionOrchestrationApi>,
    input: EvolutionQueryInput,
) -> EvolutionCommandResult {
    boundary(api.probations(
        &input.workspace_id,
        input.cursor.as_deref(),
        input.limit.unwrap_or(50),
    ))
}

#[tauri::command]
pub(crate) fn list_skill_evolution_breakers(
    api: State<'_, SkillEvolutionOrchestrationApi>,
    input: EvolutionQueryInput,
) -> EvolutionCommandResult {
    boundary(api.breakers(
        &input.workspace_id,
        input.cursor.as_deref(),
        input.limit.unwrap_or(50),
    ))
}

#[tauri::command]
pub(crate) fn request_skill_evolution_run(
    api: State<'_, SkillEvolutionOrchestrationApi>,
    workspace_id: String,
) -> EvolutionCommandResult {
    boundary(api.request_manual_run(&workspace_id, now()))
}

#[tauri::command]
pub(crate) fn cancel_skill_evolution_run(
    api: State<'_, SkillEvolutionOrchestrationApi>,
    run_id: String,
    expected_revision: u64,
) -> EvolutionCommandResult {
    boundary(api.cancel_run(&run_id, expected_revision, now()))
}

#[tauri::command]
pub(crate) fn acknowledge_skill_evolution_breaker(
    api: State<'_, SkillEvolutionOrchestrationApi>,
    breaker_id: String,
    expected_revision: u64,
) -> EvolutionCommandResult {
    boundary(api.acknowledge_breaker_projection(&breaker_id, expected_revision, now()))
}

#[tauri::command]
pub(crate) fn dispatch_skill_evolution_notifications(
    app: AppHandle,
    api: State<'_, SkillEvolutionOrchestrationApi>,
) -> EvolutionCommandResult {
    boundary(api.dispatch_notifications(now(), |event| {
        app.emit("skill-evolution-orchestration:notification", event)
            .map_err(|_| ())
    }))
}

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
