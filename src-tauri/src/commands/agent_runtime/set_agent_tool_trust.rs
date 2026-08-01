use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn set_agent_tool_trust(
    api: State<'_, AgentRuntimeApi>,
    agent_id: String,
    enabled: bool,
) -> Result<super::dto::AgentRegistryEntry, CommandError> {
    api.set_auto_approve_tools(&agent_id, enabled)
        .map(mapper::agent_to_dto)
        .map_err(map_command_error)
}
