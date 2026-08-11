use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::cli_config::{
    ApplyCliConfigProfileInput, CliConfigApi, CliConfigApplyResult,
};
use tauri::State;

#[tauri::command]
pub(crate) fn apply_cli_config_profile(
    api: State<'_, CliConfigApi>,
    input: ApplyCliConfigProfileInput,
) -> Result<CliConfigApplyResult, CommandError> {
    api.apply_profile(input).map_err(map_command_error)
}
