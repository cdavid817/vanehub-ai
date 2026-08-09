use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::task_orchestration::api::{PlanDraft, TaskOrchestrationApi};
use tauri::State;

#[tauri::command]
pub(crate) fn get_plan_draft(
    api: State<'_, TaskOrchestrationApi>,
    plan_id: String,
) -> Result<Option<PlanDraft>, CommandError> {
    api.find_plan_draft(&plan_id).map_err(map_command_error)
}
