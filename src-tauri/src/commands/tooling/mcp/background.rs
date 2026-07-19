use crate::contexts::tooling::mcp::api::{McpApi, PreparedConnectionTest};

pub(super) fn spawn_connection_test(api: McpApi, prepared: PreparedConnectionTest) {
    tauri::async_runtime::spawn(async move {
        let _ = api.execute_connection_test(prepared).await;
    });
}
