use tauri::State;

use super::background;
use super::dto::CliOperationHandleDto;
use super::error::{command_error, CliEnvironmentCommandError};
use crate::contexts::tooling::cli::api::CliEnvironmentApi;

/// Runs one tool's own read-only diagnostics.
///
/// Read-only by construction: the probes a tool declares are non-interactive and never capture a
/// credential. A tool with no documented doctor command reports `unknown`, which is a truthful
/// answer rather than a failure.
#[tauri::command]
pub(crate) fn run_cli_doctor(
    api: State<'_, CliEnvironmentApi>,
    agent_id: String,
) -> Result<CliOperationHandleDto, CliEnvironmentCommandError> {
    let prepared = api.prepare_doctor(&agent_id).map_err(command_error)?;
    let operation_id = prepared.operation_id.clone();
    background::spawn_doctor(api.inner().clone(), prepared);
    Ok(CliOperationHandleDto { operation_id })
}
