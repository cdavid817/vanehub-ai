use crate::contexts::tooling::sdk::api::{PreparedSdkOperation, SdkApi};

pub(super) fn spawn_operation(api: SdkApi, prepared: PreparedSdkOperation) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = api.execute_operation(prepared);
    });
}
