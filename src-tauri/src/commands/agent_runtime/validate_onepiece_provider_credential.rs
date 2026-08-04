use crate::commands::agent_runtime::{dto, mapper};
use crate::contexts::agent_runtime::api::{AgentRuntimeApi, ProviderCredentialValidationResult};
use tauri::State;

#[tauri::command]
pub(crate) async fn validate_onepiece_provider_credential(
    api: State<'_, AgentRuntimeApi>,
    input: dto::ValidateOnePieceProviderCredentialInput,
) -> Result<ProviderCredentialValidationResult, String> {
    api.validate_onepiece_provider_credential(
        mapper::validate_onepiece_provider_credential_request(input),
    )
    .await
    .map_err(|error| error.to_string())
}
