use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::FloatingAssistantApi;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub(crate) fn show_main_window(
    app: AppHandle,
    api: State<'_, FloatingAssistantApi>,
    action: String,
) -> Result<(), CommandError> {
    let action = mapper::floating_main_action(&action)?;
    api.show_main_window(action).map_err(map_command_error)?;
    app.emit_to(
        "main",
        "floating-assistant:event",
        dto::FloatingAssistantEvent::MainAction {
            action: action.as_str().to_string(),
        },
    )
    .map_err(|error| CommandError::storage(error.to_string()))
}
