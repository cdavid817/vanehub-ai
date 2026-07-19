use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::FloatingAssistantApi;
use tauri::State;

#[tauri::command]
pub(crate) fn save_floating_assistant_anchor(
    api: State<'_, FloatingAssistantApi>,
    anchor: dto::FloatingAssistantAnchor,
) -> Result<dto::FloatingAssistantConfig, CommandError> {
    api.save_anchor(anchor.x, anchor.y, anchor.monitor_name)
        .map(mapper::floating_config_to_dto)
        .map_err(map_command_error)
}
