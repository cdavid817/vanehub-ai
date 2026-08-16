use super::{review_dto::ReviewSessionDto, review_error::map_review_error};
use crate::commands::error::CommandError;
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_code_review(
    api: State<'_, SessionsApi>,
    review_id: String,
) -> Result<ReviewSessionDto, CommandError> {
    api.find_review(&review_id)
        .map(Into::into)
        .map_err(map_review_error)
}
