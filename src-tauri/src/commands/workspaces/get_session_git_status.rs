use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) async fn get_session_git_status(
    api: State<'_, WorkspaceApi>,
    session_id: String,
) -> Result<dto::GitStatusResult, CommandError> {
    api.get_session_git_status_blocking(session_id)
        .await
        .map(mapper::git_status_to_dto)
        .map_err(map_command_error)
}
