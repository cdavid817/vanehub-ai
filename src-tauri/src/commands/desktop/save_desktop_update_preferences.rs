use crate::contexts::desktop::{DesktopUpdateApi, UpdatePreferences};
use tauri::State;
#[tauri::command]
pub(crate) fn save_desktop_update_preferences(
    api: State<'_, DesktopUpdateApi>,
    input: UpdatePreferences,
) -> Result<UpdatePreferences, String> {
    api.save_preferences(input)
}
