use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::task_orchestration::api::{PlanControlResult, TaskOrchestrationApi};
use tauri::State;

#[tauri::command]
pub(crate) fn request_plan_control(
    api: State<'_, TaskOrchestrationApi>,
    run_id: String,
    kind: String,
) -> Result<PlanControlResult, CommandError> {
    let now = chrono::Utc::now().to_rfc3339();
    api.request_plan_control(&run_id, &kind, &now)
        .map_err(map_command_error)
}

#[tauri::command]
pub(crate) fn retry_plan_subtask(
    api: State<'_, TaskOrchestrationApi>,
    run_id: String,
    subtask_run_id: String,
) -> Result<PlanControlResult, CommandError> {
    api.retry_plan_subtask(&run_id, &subtask_run_id, &chrono::Utc::now().to_rfc3339())
        .map_err(map_command_error)
}

#[tauri::command]
pub(crate) fn accept_plan_run(
    api: State<'_, TaskOrchestrationApi>,
    run_id: String,
) -> Result<PlanControlResult, CommandError> {
    api.accept_plan_run(&run_id, &chrono::Utc::now().to_rfc3339())
        .map_err(map_command_error)
}

#[tauri::command]
pub(crate) fn recover_plan_run(
    api: State<'_, TaskOrchestrationApi>,
    run_id: String,
) -> Result<PlanControlResult, CommandError> {
    api.recover_plan_run(&run_id, &chrono::Utc::now().to_rfc3339())
        .map_err(map_command_error)
}
