use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn reset_agent_memories(api: State<'_, AgentRuntimeApi>) -> Result<(), CommandError> {
    api.reset_all_memories().map_err(map_command_error)
}
