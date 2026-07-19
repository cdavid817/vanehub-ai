use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn send_agent_terminal_input(
    api: State<'_, AgentRuntimeApi>,
    terminal_id: String,
    content: String,
) -> Result<(), CommandError> {
    api.write_agent_terminal_input(mapper::terminal_input_request(terminal_id, content))
        .map_err(map_command_error)
}
