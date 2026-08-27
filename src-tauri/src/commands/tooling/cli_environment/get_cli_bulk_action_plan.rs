use tauri::State;

use super::dto::CliBulkActionPlanDto;
use super::error::{command_error, CliEnvironmentCommandError};
use super::mapper;
use crate::contexts::tooling::cli::api::CliEnvironmentApi;

/// The batch the user is about to approve: what will run, and what was left out and why.
///
/// Direct, like its single-action counterpart.
#[tauri::command]
pub(crate) fn get_cli_bulk_action_plan(
    api: State<'_, CliEnvironmentApi>,
    plan_id: String,
) -> Result<CliBulkActionPlanDto, CliEnvironmentCommandError> {
    api.get_bulk_action_plan(&plan_id)
        .map(mapper::bulk_plan_to_dto)
        .map_err(command_error)
}
