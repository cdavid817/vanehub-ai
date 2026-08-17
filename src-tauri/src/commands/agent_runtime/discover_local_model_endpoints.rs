use super::{dto, mapper};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) async fn discover_local_model_endpoints(
    api: State<'_, AgentRuntimeApi>,
) -> Result<dto::LocalModelDiscoveryResult, String> {
    api.discover_local_model_endpoints()
        .await
        .map(mapper::local_discovery_to_dto)
        .map_err(|error| error.to_string())
}
