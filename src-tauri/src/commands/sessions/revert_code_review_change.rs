use super::review_dto::ReviewRevertReceiptDto;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use crate::contexts::workspaces::application::ReviewRevertRequest;
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RevertCodeReviewChangeInput {
    session_id: String,
    path: String,
    expected_snapshot: String,
    hunk_fingerprint: Option<String>,
    confirmed: bool,
}

#[tauri::command]
pub(crate) async fn revert_code_review_change(
    api: State<'_, WorkspaceApi>,
    input: RevertCodeReviewChangeInput,
) -> Result<ReviewRevertReceiptDto, CommandError> {
    api.revert_review_change(&ReviewRevertRequest {
        session_id: input.session_id,
        path: input.path,
        expected_snapshot: input.expected_snapshot,
        hunk_fingerprint: input.hunk_fingerprint,
        confirmed: input.confirmed,
    })
    .map(Into::into)
    .map_err(map_command_error)
}
