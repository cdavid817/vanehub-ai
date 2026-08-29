use super::review_dto::{parse_review_decision, ReviewSessionDto};
use super::review_error::map_review_error;
use crate::commands::error::CommandError;
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn set_code_review_decision(
    api: State<'_, SessionsApi>,
    review_id: String,
    decision: String,
) -> Result<ReviewSessionDto, CommandError> {
    let decision = parse_review_decision(&decision)?;
    api.set_review_decision(&review_id, decision)
        .map(Into::into)
        .map_err(map_review_error)
}
