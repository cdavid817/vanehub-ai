use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_session_directory(
    api: State<'_, WorkspaceApi>,
    session_id: String,
    path: String,
) -> Result<dto::DirectoryListing, CommandError> {
    api.list_session_directory(&session_id, &path)
        .map(mapper::directory_listing_to_dto)
        .map_err(map_command_error)
}
