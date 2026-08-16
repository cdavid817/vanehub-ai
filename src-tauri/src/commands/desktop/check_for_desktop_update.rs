use crate::contexts::desktop::{DesktopUpdateApi, UpdateReceipt};
use tauri::State;
#[tauri::command]
pub(crate) fn check_for_desktop_update(
    api: State<'_, DesktopUpdateApi>,
) -> Result<UpdateReceipt, String> {
    api.start_check()
}
