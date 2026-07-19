use crate::contexts::tooling::cli::api::{
    CliApi, PreparedCliInstall, PreparedCliRefresh, PreparedCliUpgradeAll,
};

pub(super) fn spawn_refresh(api: CliApi, prepared: PreparedCliRefresh) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = api.execute_refresh(prepared);
    });
}

pub(super) fn spawn_install(api: CliApi, prepared: PreparedCliInstall) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = api.execute_install(prepared);
    });
}

pub(super) fn spawn_upgrade_all(api: CliApi, prepared: PreparedCliUpgradeAll) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = api.execute_upgrade_all(prepared);
    });
}
