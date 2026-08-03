use super::{dto, mapper};
use crate::commands::error::CommandError;
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn reset_onepiece_provider_config(
    api: State<'_, AgentRuntimeApi>,
) -> Result<dto::OnePieceProviderConfig, CommandError> {
    api.reset_onepiece_provider_config()
        .map(mapper::onepiece_provider_config_to_dto)
        .map_err(CommandError::from)
}
