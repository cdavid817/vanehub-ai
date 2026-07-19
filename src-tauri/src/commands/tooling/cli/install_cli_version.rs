use super::{background, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::operations::api::OperationTask;
use crate::contexts::tooling::cli::api::CliApi;
use tauri::State;

#[tauri::command]
pub(crate) fn install_cli_version(
    api: State<'_, CliApi>,
    agent_id: String,
    target_version: String,
    confirmed_active_path: Option<String>,
) -> Result<OperationTask, CommandError> {
    let prepared = api
        .prepare_install(agent_id, target_version, confirmed_active_path)
        .map_err(map_command_error)?;
    let operation = mapper::started_operation_to_dto(&prepared.operation);
    background::spawn_install(api.inner().clone(), prepared);
    Ok(operation)
}
