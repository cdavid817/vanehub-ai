use super::{dto, events, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::{AppHandle, State};

#[tauri::command]
pub(crate) fn save_session_chat_config(
    app: AppHandle,
    api: State<'_, SessionsApi>,
    session_id: String,
    config: dto::ChatConfig,
) -> Result<dto::ChatConfig, CommandError> {
    let saved = api
        .save_chat_configuration(mapper::chat_configuration_request(
            session_id.clone(),
            config,
        ))
        .and_then(mapper::chat_configuration_to_dto)
        .map_err(map_command_error)?;
    events::emit_configuration_changed(&app, &session_id);
    Ok(saved)
}
