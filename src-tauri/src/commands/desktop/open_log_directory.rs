use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::DesktopSettingsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn open_log_directory(api: State<'_, DesktopSettingsApi>) -> Result<(), CommandError> {
    api.open_log_directory().map_err(map_command_error)
}
