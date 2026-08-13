use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::task_orchestration::api::{PlanDraft, TaskOrchestrationApi};
use tauri::State;

#[tauri::command]
pub(crate) fn save_plan_draft(
    api: State<'_, TaskOrchestrationApi>,
    input: PlanDraft,
) -> Result<PlanDraft, CommandError> {
    api.save_plan_draft(&input).map_err(map_command_error)
}
