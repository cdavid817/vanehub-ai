use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_session_documents(
    api: State<'_, WorkspaceApi>,
    session_id: String,
) -> Result<dto::DocumentListing, CommandError> {
    api.list_session_documents(&session_id)
        .map(mapper::document_listing_to_dto)
        .map_err(map_command_error)
}
