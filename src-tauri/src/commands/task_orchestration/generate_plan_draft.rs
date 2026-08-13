use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::task_orchestration::api::{
    GeneratePlanDraftRequest, PlanDraft, TaskOrchestrationApi,
};
use tauri::State;

#[tauri::command]
pub(crate) fn generate_plan_draft(
    api: State<'_, TaskOrchestrationApi>,
    input: GeneratePlanDraftRequest,
) -> Result<PlanDraft, CommandError> {
    api.generate_plan_draft(&input).map_err(map_command_error)
}
