use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_agents(
    api: State<'_, AgentRuntimeApi>,
    capability_tag: Option<String>,
) -> Result<Vec<dto::AgentRegistryEntry>, CommandError> {
    api.list_agents(capability_tag.as_deref())
        .map(mapper::agents_to_dto)
        .map_err(map_command_error)
}
