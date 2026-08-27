use tauri::State;

use super::background;
use super::dto::CliOperationHandleDto;
use super::error::{command_error, CliEnvironmentCommandError};
use crate::contexts::tooling::cli::api::CliEnvironmentApi;

/// Runs a prepared batch, one item at a time.
///
/// An item whose own plan has gone stale or been consumed is skipped and the batch continues; the
/// per-item outcome lands on the operation result. Returns before any of it completes.
#[tauri::command]
pub(crate) fn execute_cli_bulk_action(
    api: State<'_, CliEnvironmentApi>,
    plan_id: String,
    expected_revision: u32,
) -> Result<CliOperationHandleDto, CliEnvironmentCommandError> {
    let prepared = api
        .prepare_bulk_execution(&plan_id, expected_revision)
        .map_err(command_error)?;
    let operation_id = prepared.operation_id.clone();
    background::spawn_bulk_action(api.inner().clone(), prepared);
    Ok(CliOperationHandleDto { operation_id })
}
