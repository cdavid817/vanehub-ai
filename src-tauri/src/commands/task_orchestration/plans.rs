use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::task_orchestration::api::{
    PlanAttemptEvidenceView, PlanDraft, TaskOrchestrationApi,
};
use tauri::State;

#[tauri::command]
pub(crate) fn validate_plan_draft(
    api: State<'_, TaskOrchestrationApi>,
    input: PlanDraft,
) -> Result<(), CommandError> {
    api.validate_plan_draft(&input).map_err(map_command_error)
}

#[tauri::command]
pub(crate) fn list_plan_versions(
    api: State<'_, TaskOrchestrationApi>,
    plan_id: String,
) -> Result<Vec<PlanDraft>, CommandError> {
    api.list_plan_versions(&plan_id).map_err(map_command_error)
}

#[tauri::command]
pub(crate) fn delete_plan_draft(
    api: State<'_, TaskOrchestrationApi>,
    plan_id: String,
) -> Result<(), CommandError> {
    api.delete_plan_draft(&plan_id).map_err(map_command_error)
}

#[tauri::command]
pub(crate) fn get_plan_attempt_evidence(
    api: State<'_, TaskOrchestrationApi>,
    attempt_id: String,
) -> Result<Vec<PlanAttemptEvidenceView>, CommandError> {
    api.get_plan_attempt_evidence(&attempt_id)
        .map_err(map_command_error)
}
