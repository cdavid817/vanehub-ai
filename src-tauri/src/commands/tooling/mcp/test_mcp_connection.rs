use super::{background, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::operations::api::OperationTask;
use crate::contexts::tooling::mcp::api::McpApi;
use tauri::State;

#[tauri::command]
pub(crate) fn test_mcp_connection(
    api: State<'_, McpApi>,
    name: String,
) -> Result<OperationTask, CommandError> {
    let prepared = api
        .prepare_connection_test(&name)
        .map_err(map_command_error)?;
    let operation = mapper::started_operation_to_dto(&prepared.operation);
    background::spawn_connection_test(api.inner().clone(), prepared);
    Ok(operation)
}
