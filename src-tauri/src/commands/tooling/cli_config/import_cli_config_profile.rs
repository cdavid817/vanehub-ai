use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::cli_config::{
    CliConfigApi, CliConfigProfile, ImportCliConfigProfileInput,
};
use tauri::State;

#[tauri::command]
pub(crate) fn import_cli_config_profile(
    api: State<'_, CliConfigApi>,
    input: ImportCliConfigProfileInput,
) -> Result<CliConfigProfile, CommandError> {
    api.import_current(input).map_err(map_command_error)
}
