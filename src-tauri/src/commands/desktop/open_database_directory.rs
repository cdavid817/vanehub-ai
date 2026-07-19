use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::DesktopSettingsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn open_database_directory(
    api: State<'_, DesktopSettingsApi>,
) -> Result<(), CommandError> {
    api.open_database_directory().map_err(map_command_error)
}
