use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn update_api_agent(
    api: State<'_, AgentRuntimeApi>,
    agent_id: String,
    input: dto::UpdateApiAgentInput,
) -> Result<dto::AgentRegistryEntry, CommandError> {
    api.update_api_agent(&agent_id, mapper::update_api_agent_request(input))
        .map(mapper::agent_to_dto)
        .map_err(map_command_error)
}
