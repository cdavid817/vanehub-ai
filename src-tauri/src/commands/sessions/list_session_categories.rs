use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_session_categories(
    api: State<'_, SessionsApi>,
) -> Result<Vec<dto::SessionCategory>, CommandError> {
    api.list_categories()
        .map(mapper::categories_to_dto)
        .map_err(map_command_error)
}
