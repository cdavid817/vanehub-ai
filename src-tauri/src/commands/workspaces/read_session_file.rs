use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) fn read_session_file(
    api: State<'_, WorkspaceApi>,
    session_id: String,
    path: String,
) -> Result<dto::FileContent, CommandError> {
    api.read_session_file(&session_id, &path)
        .map(mapper::file_content_to_dto)
        .map_err(map_command_error)
}
