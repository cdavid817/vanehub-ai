use super::dto::{McpImportExport, McpImportResult, McpScope};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::mcp::api::McpApi;
use tauri::State;

#[tauri::command]
pub(crate) fn import_mcp_servers(
    api: State<'_, McpApi>,
    data: McpImportExport,
    scope: McpScope,
) -> Result<McpImportResult, CommandError> {
    api.import_servers(mapper::import_bundle(data), mapper::scope_to_domain(scope))
        .map(mapper::import_result_to_dto)
        .map_err(map_command_error)
}
