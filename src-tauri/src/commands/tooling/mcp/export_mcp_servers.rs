use super::dto::McpImportExport;
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::mcp::api::McpApi;
use tauri::State;

#[tauri::command]
pub(crate) fn export_mcp_servers(
    api: State<'_, McpApi>,
    names: Vec<String>,
) -> Result<McpImportExport, CommandError> {
    api.export_servers(names)
        .map(mapper::export_bundle_to_dto)
        .map_err(map_command_error)
}
