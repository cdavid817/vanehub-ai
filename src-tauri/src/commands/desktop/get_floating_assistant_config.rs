use super::{dto::FloatingAssistantConfig, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::FloatingAssistantApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_floating_assistant_config(
    api: State<'_, FloatingAssistantApi>,
) -> Result<FloatingAssistantConfig, CommandError> {
    api.get_config()
        .map(mapper::floating_config_to_dto)
        .map_err(map_command_error)
}
