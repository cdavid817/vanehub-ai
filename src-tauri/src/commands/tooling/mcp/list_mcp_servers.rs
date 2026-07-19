use super::dto::McpServerConfig;
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::mcp::api::McpApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_mcp_servers(
    api: State<'_, McpApi>,
) -> Result<Vec<McpServerConfig>, CommandError> {
    api.list_servers()
        .map(mapper::servers_to_dto)
        .map_err(map_command_error)
}
