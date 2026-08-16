use crate::contexts::desktop::{DesktopUpdateApi, UpdatePreferences};
use tauri::State;
#[tauri::command]
pub(crate) fn get_desktop_update_preferences(
    api: State<'_, DesktopUpdateApi>,
) -> Result<UpdatePreferences, String> {
    api.preferences()
}
