use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::task_orchestration::api::{
    PlanRunDetailView, PlanRunPageView, PlanRunSummaryView, TaskOrchestrationApi,
};
use tauri::State;

#[tauri::command]
pub(crate) fn list_plan_runs(
    api: State<'_, TaskOrchestrationApi>,
    cursor: Option<String>,
) -> Result<PlanRunPageView, CommandError> {
    api.list_plan_runs(cursor.as_deref())
        .map_err(map_command_error)
}

#[tauri::command]
pub(crate) fn get_plan_run_for_session(
    api: State<'_, TaskOrchestrationApi>,
    session_id: String,
) -> Result<Option<PlanRunSummaryView>, CommandError> {
    api.find_plan_run_for_session(&session_id)
        .map_err(map_command_error)
}

#[tauri::command]
pub(crate) fn get_plan_run_detail(
    api: State<'_, TaskOrchestrationApi>,
    run_id: String,
) -> Result<PlanRunDetailView, CommandError> {
    api.get_plan_run(&run_id).map_err(map_command_error)
}
