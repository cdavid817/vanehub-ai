use super::dto::{ApprovalTransitionSummary, ApprovePlanResult};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::task_orchestration::api::TaskOrchestrationApi;
use tauri::State;

#[tauri::command]
pub(crate) fn approve_plan(
    api: State<'_, TaskOrchestrationApi>,
    plan_id: String,
    originating_session_id: Option<String>,
) -> Result<ApprovePlanResult, CommandError> {
    let run_id = api
        .approve_plan(
            &plan_id,
            originating_session_id.as_deref(),
            &chrono::Utc::now().to_rfc3339(),
        )
        .map_err(map_command_error)?;
    let run = api.get_plan_run(&run_id).map_err(map_command_error)?;
    Ok(ApprovePlanResult {
        run_id,
        summary: ApprovalTransitionSummary {
            project_path: run.project_path,
            task_count: run.tasks.len(),
            retained_worktree: true,
            automatic_git_operations: false,
        },
    })
}
