use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::{
    DesktopSettingsApi, FolderOpenerPreferencesView, SaveFolderOpenerPreferences,
};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub(crate) fn save_folder_opener_preferences(
    app: AppHandle,
    api: State<'_, DesktopSettingsApi>,
    input: SaveFolderOpenerPreferences,
) -> Result<FolderOpenerPreferencesView, CommandError> {
    let result = api
        .save_folder_opener_preferences(input)
        .map_err(map_command_error)?;
    app.emit("folder-openers:event", "preferences-changed")
        .map_err(|error| CommandError::storage(error.to_string()))?;
    Ok(result)
}
