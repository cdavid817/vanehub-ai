use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn stop_agent_terminal(
    api: State<'_, AgentRuntimeApi>,
    terminal_id: String,
) -> Result<bool, CommandError> {
    api.stop_agent_terminal(mapper::stop_terminal_request(terminal_id))
        .map_err(map_command_error)
}
