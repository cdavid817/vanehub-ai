use super::{dto, mapper};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) async fn verify_local_model_endpoint(
    api: State<'_, AgentRuntimeApi>,
    input: dto::LocalEndpointVerificationRequest,
) -> Result<dto::LocalModelDiscoveryResult, String> {
    api.verify_local_model_endpoint(mapper::local_verification_request(input))
        .await
        .map(mapper::local_discovery_to_dto)
        .map_err(|error| error.to_string())
}
