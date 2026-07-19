use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::DesktopSettingsApi;
use tauri::State;

#[tauri::command]
pub(crate) async fn test_network_proxy(
    api: State<'_, DesktopSettingsApi>,
    input: dto::TestNetworkProxyInput,
) -> Result<dto::NetworkProxyTestResult, CommandError> {
    let api = api.inner().clone();
    api.test_network_proxy(input.url, input.bypass)
        .await
        .map(mapper::proxy_test_to_dto)
        .map_err(map_command_error)
}
