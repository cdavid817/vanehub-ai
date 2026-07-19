use super::{background, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::operations::api::OperationTask;
use crate::contexts::tooling::cli::api::CliApi;
use tauri::State;

#[tauri::command]
pub(crate) fn refresh_cli_detections(
    api: State<'_, CliApi>,
    agent_id: Option<String>,
) -> Result<OperationTask, CommandError> {
    let prepared = api
        .prepare_refresh(agent_id, "Refreshing CLI detections".to_string())
        .map_err(map_command_error)?;
    let operation = mapper::started_operation_to_dto(&prepared.operation);
    background::spawn_refresh(api.inner().clone(), prepared);
    Ok(operation)
}
