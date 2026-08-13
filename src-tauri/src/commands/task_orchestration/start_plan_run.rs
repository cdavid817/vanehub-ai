use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::task_orchestration::api::{PreparedPlanRun, TaskOrchestrationApi};
use tauri::State;

#[tauri::command]
pub(crate) fn start_plan_run(
    api: State<'_, TaskOrchestrationApi>,
    run_id: String,
) -> Result<PreparedPlanRun, CommandError> {
    api.prepare_plan_run(&run_id, &chrono::Utc::now().to_rfc3339())
        .map_err(map_command_error)
}
