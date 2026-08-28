use tauri::State;

use super::dto::CliActionPlanDto;
use super::error::{command_error, CliEnvironmentCommandError};
use super::mapper;
use crate::contexts::tooling::cli::api::CliEnvironmentApi;

/// The plan the user is about to approve, including the exact argv that will run.
///
/// Direct: a persisted plan is a stored row, and reviewing one starts nothing.
#[tauri::command]
pub(crate) fn get_cli_action_plan(
    api: State<'_, CliEnvironmentApi>,
    plan_id: String,
) -> Result<CliActionPlanDto, CliEnvironmentCommandError> {
    api.get_action_plan(&plan_id)
        .map(mapper::plan_to_dto)
        .map_err(command_error)
}
