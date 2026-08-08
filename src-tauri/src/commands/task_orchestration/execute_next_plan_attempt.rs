use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::task_orchestration::api::{ExecutedPlanAttempt, TaskOrchestrationApi};
use tauri::State;

#[tauri::command]
pub(crate) fn execute_next_plan_attempt(
    api: State<'_, TaskOrchestrationApi>,
    run_id: String,
) -> Result<Option<ExecutedPlanAttempt>, CommandError> {
    api.execute_next_attempt(&run_id, &chrono::Utc::now().to_rfc3339())
        .map_err(map_command_error)
}
