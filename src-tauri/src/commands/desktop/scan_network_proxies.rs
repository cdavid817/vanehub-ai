use super::{dto, mapper};
use crate::commands::error::CommandError;
use crate::contexts::desktop::api::DesktopSettingsApi;
use tauri::State;

#[tauri::command]
pub(crate) async fn scan_network_proxies(
    api: State<'_, DesktopSettingsApi>,
) -> Result<Vec<dto::DetectedNetworkProxy>, CommandError> {
    let api = api.inner().clone();
    Ok(api
        .scan_network_proxies()
        .await
        .into_iter()
        .map(mapper::detected_proxy_to_dto)
        .collect())
}
