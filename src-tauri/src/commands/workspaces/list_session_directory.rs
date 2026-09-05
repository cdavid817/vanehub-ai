use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

/// One page of a directory.
///
/// The cursor is optional and absent means "from the start", which is what every first request is.
/// A cursor that no longer applies comes back as a page with no entries and a reason on its
/// coverage, not as an error — the caller has to be able to tell "start this listing again" from
/// "this workspace is unreachable", and only one of those is worth retrying.
#[tauri::command]
pub(crate) async fn list_session_directory(
    api: State<'_, WorkspaceApi>,
    session_id: String,
    path: String,
    cursor: Option<String>,
    limit: Option<usize>,
) -> Result<dto::DirectoryListing, CommandError> {
    api.list_session_directory_blocking(session_id, path, cursor, limit)
        .await
        .map(mapper::directory_listing_to_dto)
        .map_err(map_command_error)
}
