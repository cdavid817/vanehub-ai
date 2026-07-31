use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn delete_agent_memory(
    api: State<'_, AgentRuntimeApi>,
    memory_id: String,
) -> Result<(), CommandError> {
    api.delete_agent_memory(&memory_id)
        .map_err(map_command_error)
}
