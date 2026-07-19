use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::cli_parameters::{CliParameterProfile, CliParametersApi};
use tauri::State;

#[tauri::command]
pub(crate) fn list_cli_parameter_profiles(
    api: State<'_, CliParametersApi>,
) -> Result<Vec<CliParameterProfile>, CommandError> {
    api.list_profiles().map_err(map_command_error)
}
