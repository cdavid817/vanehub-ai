use crate::contexts::desktop::{infrastructure::UpdateSnapshot, DesktopUpdateApi};
use tauri::State;
#[tauri::command]
pub(crate) fn get_desktop_update_snapshot(
    api: State<'_, DesktopUpdateApi>,
) -> Result<UpdateSnapshot, String> {
    api.snapshot()
}
