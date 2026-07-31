use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_api_agent_provider_config(
    api: State<'_, AgentRuntimeApi>,
    agent_id: String,
) -> Result<Option<dto::ApiAgentProviderConfig>, CommandError> {
    api.api_agent_provider_config(&agent_id)
        .map(|config| config.map(mapper::api_agent_provider_config_to_dto))
        .map_err(map_command_error)
}
