use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::FloatingAssistantApi;
use tauri::State;

#[tauri::command]
pub(crate) fn start_floating_assistant_drag(
    api: State<'_, FloatingAssistantApi>,
) -> Result<(), CommandError> {
    api.start_dragging().map_err(map_command_error)
}
