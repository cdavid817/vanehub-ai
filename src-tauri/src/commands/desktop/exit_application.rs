use crate::contexts::desktop::api::DesktopLifecycleApi;
use tauri::State;

#[tauri::command]
pub(crate) fn exit_application(api: State<'_, DesktopLifecycleApi>) {
    #[cfg(feature = "desktop-e2e")]
    {
        let api = api.inner().clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            api.request_exit();
        });
        // No `return` needed: the branch below is compiled out under this feature, and clippy
        // rejects the redundancy once the feature is linted.
    }

    #[cfg(not(feature = "desktop-e2e"))]
    api.request_exit();
}
