use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::mcp::api::McpApi;
use tauri::State;

#[tauri::command]
pub(crate) fn remove_mcp_server(api: State<'_, McpApi>, name: String) -> Result<(), CommandError> {
    api.remove_server(&name).map_err(map_command_error)
}
