use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_agent_by_id(
    api: State<'_, AgentRuntimeApi>,
    agent_id: String,
) -> Result<dto::AgentRegistryEntry, CommandError> {
    api.get_agent(&agent_id)
        .map(mapper::agent_to_dto)
        .map_err(map_command_error)
}
