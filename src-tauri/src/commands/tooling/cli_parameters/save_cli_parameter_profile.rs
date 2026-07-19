use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::cli_parameters::{
    CliParameterProfile, CliParametersApi, SaveCliParameterProfileInput,
};
use tauri::State;

#[tauri::command]
pub(crate) fn save_cli_parameter_profile(
    api: State<'_, CliParametersApi>,
    input: SaveCliParameterProfileInput,
) -> Result<CliParameterProfile, CommandError> {
    api.save_profile(&input).map_err(map_command_error)
}
