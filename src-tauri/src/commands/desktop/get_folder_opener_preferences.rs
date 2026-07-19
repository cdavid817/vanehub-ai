use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::{DesktopSettingsApi, FolderOpenerPreferencesView};
use tauri::State;

#[tauri::command]
pub(crate) fn get_folder_opener_preferences(
    api: State<'_, DesktopSettingsApi>,
) -> Result<FolderOpenerPreferencesView, CommandError> {
    api.get_folder_opener_preferences()
        .map_err(map_command_error)
}
