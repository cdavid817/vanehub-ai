use super::dto::McpServerConfig;
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::mcp::api::McpApi;
use tauri::State;

#[tauri::command]
pub(crate) fn add_mcp_server(
    api: State<'_, McpApi>,
    config: McpServerConfig,
) -> Result<(), CommandError> {
    api.add_server(mapper::server_draft(config))
        .map_err(map_command_error)
}
