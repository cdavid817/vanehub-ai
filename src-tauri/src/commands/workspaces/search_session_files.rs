use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

const DEFAULT_MAX_RESULTS: usize = 8;

#[tauri::command]
pub(crate) async fn search_session_files(
    api: State<'_, WorkspaceApi>,
    session_id: String,
    query: String,
    max_results: Option<usize>,
) -> Result<dto::FileSearchListing, CommandError> {
    api.search_session_files_blocking(
        session_id,
        query,
        max_results.unwrap_or(DEFAULT_MAX_RESULTS),
    )
    .await
    .map(mapper::file_search_listing_to_dto)
    .map_err(map_command_error)
}
