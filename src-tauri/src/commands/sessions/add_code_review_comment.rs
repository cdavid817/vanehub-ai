use super::review_dto::{ReviewAnchorInput, ReviewCommentDto};
use super::review_error::map_review_error;
use crate::commands::error::CommandError;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::sessions::application::AddReviewCommentRequest;
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddCodeReviewCommentInput {
    review_id: String,
    anchor: ReviewAnchorInput,
    body: String,
}

#[tauri::command]
pub(crate) fn add_code_review_comment(
    api: State<'_, SessionsApi>,
    input: AddCodeReviewCommentInput,
) -> Result<ReviewCommentDto, CommandError> {
    let anchor = input
        .anchor
        .into_domain()
        .map_err(CommandError::validation)?;
    api.add_review_comment(AddReviewCommentRequest {
        review_id: input.review_id,
        anchor,
        body: input.body,
    })
    .map(Into::into)
    .map_err(map_review_error)
}
