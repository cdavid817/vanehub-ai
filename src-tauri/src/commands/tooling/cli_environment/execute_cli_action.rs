use tauri::State;

use super::background;
use super::dto::CliOperationHandleDto;
use super::error::{command_error, CliEnvironmentCommandError};
use crate::contexts::tooling::cli::api::CliEnvironmentApi;
use crate::contexts::tooling::cli::application::environment_planning::ExecuteCliActionInput;

/// Runs a prepared plan.
///
/// Takes a plan id and the revision the user saw, and nothing else. There is no parameter here a
/// command could be rebuilt from, so the version the user selected cannot be dropped between review
/// and execution -- and a plan revised in between is refused rather than run.
///
/// Returns before the package manager, the download, or the child process completes.
#[tauri::command]
pub(crate) fn execute_cli_action(
    api: State<'_, CliEnvironmentApi>,
    plan_id: String,
    expected_revision: u32,
) -> Result<CliOperationHandleDto, CliEnvironmentCommandError> {
    let prepared = api
        .prepare_action_execution(ExecuteCliActionInput {
            plan_id,
            expected_revision,
        })
        .map_err(command_error)?;
    let operation_id = prepared.operation_id.clone();
    background::spawn_action(api.inner().clone(), prepared);
    Ok(CliOperationHandleDto { operation_id })
}
