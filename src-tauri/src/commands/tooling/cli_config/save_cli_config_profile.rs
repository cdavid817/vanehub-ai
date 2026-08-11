use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::cli_config::{
    CliConfigApi, CliConfigProfile, SaveCliConfigProfileInput,
};
use tauri::State;

#[tauri::command]
pub(crate) fn save_cli_config_profile(
    api: State<'_, CliConfigApi>,
    input: SaveCliConfigProfileInput,
) -> Result<CliConfigProfile, CommandError> {
    api.save_profile(input).map_err(map_command_error)
}
