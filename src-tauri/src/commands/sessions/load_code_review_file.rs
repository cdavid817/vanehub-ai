use super::review_dto::ReviewDiffFileDto;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) async fn load_code_review_file(
    api: State<'_, WorkspaceApi>,
    session_id: String,
    path: String,
    expected_snapshot: String,
) -> Result<ReviewDiffFileDto, CommandError> {
    api.load_review_file(&session_id, &path, &expected_snapshot)
        .map(Into::into)
        .map_err(map_command_error)
}
