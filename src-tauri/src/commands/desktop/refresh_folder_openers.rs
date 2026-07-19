use crate::contexts::desktop::api::{DesktopSettingsApi, FolderOpenerAvailability};
use tauri::State;

#[tauri::command]
pub(crate) fn refresh_folder_openers(
    api: State<'_, DesktopSettingsApi>,
) -> Vec<FolderOpenerAvailability> {
    api.list_folder_openers(true)
}
