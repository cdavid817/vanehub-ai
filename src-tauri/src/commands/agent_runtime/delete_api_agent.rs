use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn delete_api_agent(
    api: State<'_, AgentRuntimeApi>,
    agent_id: String,
) -> Result<(), CommandError> {
    api.delete_api_agent(&agent_id).map_err(map_command_error)
}
