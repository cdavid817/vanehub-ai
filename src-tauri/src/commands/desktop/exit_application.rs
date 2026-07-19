use crate::contexts::desktop::api::DesktopLifecycleApi;
use tauri::State;

#[tauri::command]
pub(crate) fn exit_application(api: State<'_, DesktopLifecycleApi>) {
    api.request_exit();
}
