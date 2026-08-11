use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::cli_config::{CliConfigApi, DeleteCliConfigProfileInput};
use tauri::State;

#[tauri::command]
pub(crate) fn delete_cli_config_profile(
    api: State<'_, CliConfigApi>,
    input: DeleteCliConfigProfileInput,
) -> Result<(), CommandError> {
    api.delete_profile(input).map_err(map_command_error)
}
