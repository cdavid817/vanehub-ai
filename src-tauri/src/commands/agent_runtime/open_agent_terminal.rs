use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn open_agent_terminal(
    api: State<'_, AgentRuntimeApi>,
    session_id: String,
    size: dto::AgentTerminalSize,
) -> Result<dto::AgentTerminalSession, CommandError> {
    api.open_agent_terminal(mapper::open_agent_terminal_request(session_id, size))
        .map(mapper::terminal_session_to_dto)
        .map_err(map_command_error)
}
