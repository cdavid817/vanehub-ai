use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) async fn list_loop_branches(
    api: State<'_, WorkspaceApi>,
    project_path: String,
) -> Result<Vec<dto::LoopBranchChoice>, CommandError> {
    api.list_git_branches_blocking(project_path)
        .await
        .map(|branches| branches.into_iter().map(mapper::branch).collect())
        .map_err(map_command_error)
}
