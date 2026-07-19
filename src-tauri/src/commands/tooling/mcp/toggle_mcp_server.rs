use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::mcp::api::McpApi;
use tauri::State;

#[tauri::command]
pub(crate) fn toggle_mcp_server(
    api: State<'_, McpApi>,
    name: String,
    active: bool,
) -> Result<(), CommandError> {
    api.toggle_server(&name, active).map_err(map_command_error)
}
