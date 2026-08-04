use super::{dto, mapper};
use crate::commands::error::CommandError;
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn save_onepiece_provider_config(
    api: State<'_, AgentRuntimeApi>,
    input: dto::SaveOnePieceProviderConfigInput,
) -> Result<dto::OnePieceProviderConfig, CommandError> {
    api.save_onepiece_provider_config(mapper::save_onepiece_provider_config_request(input))
        .map(mapper::onepiece_provider_config_to_dto)
        .map_err(CommandError::from)
}
