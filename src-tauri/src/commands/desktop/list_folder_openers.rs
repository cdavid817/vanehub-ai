use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::{DesktopSettingsApi, FolderOpenerAvailability};
use tauri::State;

#[tauri::command]
pub(crate) async fn list_folder_openers(
    api: State<'_, DesktopSettingsApi>,
) -> Result<Vec<FolderOpenerAvailability>, CommandError> {
    let api = api.inner().clone();
    api.list_folder_openers_detached(false)
        .await
        .map_err(map_command_error)
}
