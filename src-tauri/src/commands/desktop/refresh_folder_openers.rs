use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::{DesktopSettingsApi, FolderOpenerAvailability};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub(crate) async fn refresh_folder_openers(
    app: AppHandle,
    api: State<'_, DesktopSettingsApi>,
) -> Result<Vec<FolderOpenerAvailability>, CommandError> {
    let api = api.inner().clone();
    let detected = api
        .list_folder_openers_detached(true)
        .await
        .map_err(map_command_error)?;
    // The session toolbar caches the same list; without this it keeps showing the pre-refresh
    // catalog until a preference is saved or the app restarts.
    app.emit("folder-openers:event", "availability-changed")
        .map_err(|error| CommandError::storage(error.to_string()))?;
    Ok(detected)
}
