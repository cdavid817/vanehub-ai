use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_session_git_diff(
    api: State<'_, WorkspaceApi>,
    session_id: String,
    path: String,
    source: dto::GitDiffSource,
) -> Result<dto::GitDiffResult, CommandError> {
    api.get_session_git_diff(&session_id, &path, mapper::git_diff_source_from_dto(source))
        .map(mapper::git_diff_to_dto)
        .map_err(map_command_error)
}
