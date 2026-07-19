use super::dto::McpServerStatus;
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::mcp::api::McpApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_mcp_server_status(
    api: State<'_, McpApi>,
    name: String,
) -> Result<McpServerStatus, CommandError> {
    api.server_status(&name)
        .map(mapper::status_to_dto)
        .map_err(map_command_error)
}
