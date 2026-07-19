use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn delete_session_category(
    api: State<'_, SessionsApi>,
    category_id: String,
) -> Result<(), CommandError> {
    api.delete_category(&category_id).map_err(map_command_error)
}
