use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn rename_session_category(
    api: State<'_, SessionsApi>,
    category_id: String,
    name: String,
) -> Result<dto::SessionCategory, CommandError> {
    api.rename_category(&category_id, name)
        .map(mapper::category_to_dto)
        .map_err(map_command_error)
}
