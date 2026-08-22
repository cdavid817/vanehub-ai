use tauri::State;

use super::background;
use super::dto::CliOperationHandleDto;
use super::error::{command_error, CliEnvironmentCommandError};
use crate::contexts::tooling::cli::api::CliEnvironmentApi;

/// Prepares one bulk upgrade plan covering the requested tools.
///
/// Tools that cannot be included are recorded on the plan as skips with a reason. A silently
/// shorter item list would read as "everything else is already up to date".
#[tauri::command]
pub(crate) fn prepare_cli_bulk_action(
    api: State<'_, CliEnvironmentApi>,
    agent_ids: Vec<String>,
) -> Result<CliOperationHandleDto, CliEnvironmentCommandError> {
    let prepared = api
        .prepare_bulk_planning(agent_ids)
        .map_err(command_error)?;
    let operation_id = prepared.operation_id.clone();
    background::spawn_bulk_planning(api.inner().clone(), prepared);
    Ok(CliOperationHandleDto { operation_id })
}
