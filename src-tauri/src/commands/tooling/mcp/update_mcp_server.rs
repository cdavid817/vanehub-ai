use super::dto::PartialMcpServerConfig;
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::mcp::api::McpApi;
use tauri::State;

#[tauri::command]
pub(crate) fn update_mcp_server(
    api: State<'_, McpApi>,
    name: String,
    config: PartialMcpServerConfig,
) -> Result<(), CommandError> {
    api.update_server(&name, mapper::server_patch(config))
        .map_err(map_command_error)
}
