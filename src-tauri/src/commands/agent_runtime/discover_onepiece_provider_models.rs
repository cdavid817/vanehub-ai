use crate::commands::agent_runtime::{dto, mapper};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) async fn discover_onepiece_provider_models(
    api: State<'_, AgentRuntimeApi>,
    input: dto::DiscoverOnePieceProviderModelsInput,
) -> Result<dto::OnePieceProviderModelDiscoveryResult, String> {
    api.discover_onepiece_provider_models(mapper::discover_onepiece_provider_models_request(input))
        .await
        .map(mapper::onepiece_provider_model_discovery_to_dto)
        .map_err(|error| error.to_string())
}
