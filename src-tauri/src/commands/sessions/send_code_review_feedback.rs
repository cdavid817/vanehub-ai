use super::review_error::map_review_error;
use crate::commands::error::CommandError;
use crate::contexts::sessions::api::SessionsApi;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SendCodeReviewFeedbackResult {
    message_id: String,
}

#[tauri::command]
pub(crate) fn send_code_review_feedback(
    api: State<'_, SessionsApi>,
    review_id: String,
    acknowledge_stale: bool,
) -> Result<SendCodeReviewFeedbackResult, CommandError> {
    api.send_review_feedback(&review_id, acknowledge_stale)
        .map(|message_id| SendCodeReviewFeedbackResult { message_id })
        .map_err(map_review_error)
}
