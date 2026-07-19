use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::cli_parameters::{CliParameterProfile, CliParametersApi};
use tauri::State;

#[tauri::command]
pub(crate) fn reset_cli_parameter_profile(
    api: State<'_, CliParametersApi>,
    agent_id: String,
) -> Result<CliParameterProfile, CommandError> {
    api.reset_profile(&agent_id).map_err(map_command_error)
}
