use super::{background, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::operations::api::OperationTask;
use crate::contexts::tooling::cli::api::CliApi;
use tauri::State;

#[tauri::command]
pub(crate) fn upgrade_all_cli_versions(
    api: State<'_, CliApi>,
) -> Result<OperationTask, CommandError> {
    let prepared = api.prepare_upgrade_all().map_err(map_command_error)?;
    let operation = mapper::started_operation_to_dto(&prepared.operation);
    background::spawn_upgrade_all(api.inner().clone(), prepared);
    Ok(operation)
}
