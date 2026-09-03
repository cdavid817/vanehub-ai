use super::review_dto::ReviewFileViewedReceiptDto;
use super::review_error::map_review_error;
use crate::commands::error::CommandError;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::sessions::application::SetFileViewedRequest;
use serde::Deserialize;
use tauri::State;

/// Whether the reviewer has read a file, and the diff they read it in.
///
/// No witness comes from the caller. The witness is derived from the review's own copy of the
/// file, so a mark can never claim to be about a version of the file the review does not hold.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetCodeReviewFileViewedInput {
    review_id: String,
    relative_path: String,
    expected_snapshot_fingerprint: String,
    viewed: bool,
}

/// Records that a file has been read. Touches no decision, no Git index, and no working tree.
#[tauri::command]
pub(crate) fn set_code_review_file_viewed(
    api: State<'_, SessionsApi>,
    input: SetCodeReviewFileViewedInput,
) -> Result<ReviewFileViewedReceiptDto, CommandError> {
    let review_id = input.review_id;
    api.set_review_file_viewed(
        &review_id,
        SetFileViewedRequest {
            path: input.relative_path,
            expected_snapshot_fingerprint: input.expected_snapshot_fingerprint,
            viewed: input.viewed,
        },
    )
    .map(|state| ReviewFileViewedReceiptDto::recorded(review_id, state))
    .map_err(map_review_error)
}
