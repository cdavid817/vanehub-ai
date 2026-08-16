use super::review_error::map_review_error;
use crate::commands::error::CommandError;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::sessions::application::ReviewAction;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartCodeReviewActionResult {
    operation_id: String,
}

fn parse_review_action(action: &str) -> Result<ReviewAction, CommandError> {
    match action {
        "review-agent" => Ok(ReviewAction::ReviewAgent),
        "tests" => Ok(ReviewAction::Tests),
        "security" => Ok(ReviewAction::Security),
        _ => Err(CommandError::validation("invalid review action")),
    }
}

#[tauri::command]
pub(crate) fn start_code_review_action(
    api: State<'_, SessionsApi>,
    review_id: String,
    action: String,
) -> Result<StartCodeReviewActionResult, CommandError> {
    let action = parse_review_action(&action)?;
    api.start_review_action(&review_id, action)
        .map(|operation_id| StartCodeReviewActionResult { operation_id })
        .map_err(map_review_error)
}
