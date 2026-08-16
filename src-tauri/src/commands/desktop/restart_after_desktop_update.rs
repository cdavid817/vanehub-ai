use crate::contexts::desktop::DesktopUpdateApi;
use tauri::State;
#[tauri::command]
pub(crate) fn restart_after_desktop_update(api: State<'_, DesktopUpdateApi>) -> Result<(), String> {
    api.restart()
}
