use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) async fn list_session_directory(
    api: State<'_, WorkspaceApi>,
    session_id: String,
    path: String,
) -> Result<dto::DirectoryListing, CommandError> {
    api.list_session_directory_blocking(session_id, path)
        .await
        .map(mapper::directory_listing_to_dto)
        .map_err(map_command_error)
}
