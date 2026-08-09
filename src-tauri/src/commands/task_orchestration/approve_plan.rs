use super::dto::ApprovePlanResult;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::task_orchestration::api::TaskOrchestrationApi;
use tauri::State;

#[tauri::command]
pub(crate) fn approve_plan(
    api: State<'_, TaskOrchestrationApi>,
    plan_id: String,
) -> Result<ApprovePlanResult, CommandError> {
    api.approve_plan(&plan_id, &chrono::Utc::now().to_rfc3339())
        .map(|run_id| ApprovePlanResult { run_id })
        .map_err(map_command_error)
}
