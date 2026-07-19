use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn resize_agent_terminal(
    api: State<'_, AgentRuntimeApi>,
    terminal_id: String,
    size: dto::AgentTerminalSize,
) -> Result<(), CommandError> {
    api.resize_agent_terminal(mapper::resize_terminal_request(terminal_id, size))
        .map_err(map_command_error)
}
