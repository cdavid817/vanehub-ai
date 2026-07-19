use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_session_chat_config(
    api: State<'_, SessionsApi>,
    session_id: String,
) -> Result<dto::ChatConfig, CommandError> {
    api.load_chat_configuration(&session_id)
        .and_then(mapper::chat_configuration_to_dto)
        .map_err(map_command_error)
}
