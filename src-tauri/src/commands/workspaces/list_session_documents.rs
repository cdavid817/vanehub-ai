use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

/// Every Markdown and text file in the project.
///
/// The search id comes from the caller for the same reason it does for the searches: a recursive
/// walk of an entire project is work somebody may want to stop, and an id this side generated would
/// arrive with the answer — which is exactly too late.
#[tauri::command]
pub(crate) async fn list_session_documents(
    api: State<'_, WorkspaceApi>,
    session_id: String,
    search_id: String,
) -> Result<dto::DocumentListing, CommandError> {
    api.list_session_documents(session_id, search_id)
        .await
        .map(mapper::document_listing_to_dto)
        .map_err(map_command_error)
}
