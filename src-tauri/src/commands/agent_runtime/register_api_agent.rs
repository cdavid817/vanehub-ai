use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn register_api_agent(
    api: State<'_, AgentRuntimeApi>,
    input: dto::RegisterApiAgentInput,
) -> Result<dto::AgentRegistryEntry, CommandError> {
    api.register_api_agent(mapper::register_api_agent_request(input))
        .map(mapper::agent_to_dto)
        .map_err(map_command_error)
}
