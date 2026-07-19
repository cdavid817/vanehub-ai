use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn assign_session_category(
    api: State<'_, SessionsApi>,
    session_id: String,
    category_id: Option<String>,
) -> Result<dto::Session, CommandError> {
    api.assign_category(&session_id, category_id.as_deref())
        .and_then(mapper::session_to_dto)
        .map_err(map_command_error)
}
