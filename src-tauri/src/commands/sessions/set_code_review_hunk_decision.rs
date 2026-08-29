use super::review_dto::{parse_review_decision, ReviewHunkDecisionReceiptDto};
use super::review_error::map_review_error;
use crate::commands::error::CommandError;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::sessions::application::SetHunkDecisionRequest;
use serde::Deserialize;
use tauri::State;

/// What the reviewer decided, and the diff they were looking at when they decided it.
///
/// `expected_snapshot_fingerprint` is not optional and has no default. A decision that arrived
/// without one would be recorded against whatever the diff happens to be at that moment, which is
/// a decision the reviewer never made and which nothing downstream can tell apart from one they
/// did.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetCodeReviewHunkDecisionInput {
    review_id: String,
    relative_path: String,
    hunk_fingerprint: String,
    expected_snapshot_fingerprint: String,
    decision: String,
}

/// Records a decision about one hunk. Never touches the review's own decision, the Git index, or
/// the working tree.
#[tauri::command]
pub(crate) fn set_code_review_hunk_decision(
    api: State<'_, SessionsApi>,
    input: SetCodeReviewHunkDecisionInput,
) -> Result<ReviewHunkDecisionReceiptDto, CommandError> {
    let review_id = input.review_id;
    let decision = parse_review_decision(&input.decision)?;
    api.set_review_hunk_decision(
        &review_id,
        SetHunkDecisionRequest {
            path: input.relative_path,
            hunk_fingerprint: input.hunk_fingerprint,
            expected_snapshot_fingerprint: input.expected_snapshot_fingerprint,
            decision,
        },
    )
    .map(|recorded| ReviewHunkDecisionReceiptDto::recorded(review_id, recorded))
    .map_err(map_review_error)
}
