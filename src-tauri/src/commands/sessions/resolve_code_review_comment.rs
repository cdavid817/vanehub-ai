use super::review_dto::ReviewSessionDto;
use super::review_error::map_review_error;
use crate::commands::error::CommandError;
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn resolve_code_review_comment(
    api: State<'_, SessionsApi>,
    review_id: String,
    comment_id: String,
) -> Result<ReviewSessionDto, CommandError> {
    api.resolve_review_comment(&review_id, &comment_id)
        .map(Into::into)
        .map_err(map_review_error)
}
