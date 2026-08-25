use crate::contexts::desktop::api::DesktopLifecycleApi;
use tauri::State;

#[cfg(feature = "desktop-e2e")]
const WEBDRIVER_SESSION_SHUTDOWN_GRACE: std::time::Duration = std::time::Duration::from_secs(2);

#[tauri::command]
pub(crate) fn exit_application(api: State<'_, DesktopLifecycleApi>) {
    #[cfg(feature = "desktop-e2e")]
    {
        let api = api.inner().clone();
        std::thread::spawn(move || {
            // WDIO deletes its WebDriver session after the suite hook returns. macOS tears down
            // the embedded driver with the application, so exiting sooner races that request.
            std::thread::sleep(WEBDRIVER_SESSION_SHUTDOWN_GRACE);
            api.request_exit();
        });
        return;
    }

    #[cfg(not(feature = "desktop-e2e"))]
    api.request_exit();
}
