use super::review_dto::ReviewPatchDto;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use crate::contexts::workspaces::application::ReviewPatchRequest;
use serde::Deserialize;
use tauri::State;

/// What to render a patch for, and the diff the reviewer was looking at.
///
/// `hunk_fingerprint` absent means the whole file. Absent rather than a sentinel: "every hunk" and
/// "the hunk called empty string" are different requests, and a renderer that could not tell them
/// apart would silently produce the wrong one.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetCodeReviewPatchInput {
    session_id: String,
    path: String,
    expected_snapshot: String,
    hunk_fingerprint: Option<String>,
}

/// Renders a standard patch. Reads only: no index, no working tree, no review row is touched.
#[tauri::command]
pub(crate) async fn get_code_review_patch(
    api: State<'_, WorkspaceApi>,
    input: GetCodeReviewPatchInput,
) -> Result<ReviewPatchDto, CommandError> {
    api.render_review_patch(&ReviewPatchRequest {
        session_id: input.session_id,
        path: input.path,
        expected_snapshot: input.expected_snapshot,
        hunk_fingerprint: input.hunk_fingerprint,
    })
    .map(Into::into)
    .map_err(map_command_error)
}
