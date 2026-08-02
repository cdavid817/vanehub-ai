use crate::contexts::tooling::cli_config::{
    ApplyCliConfigProfileInput, CliConfigApi, CliConfigApplyResult,
};
use tauri::State;

#[tauri::command]
pub(crate) fn apply_cli_config_profile(
    api: State<'_, CliConfigApi>,
    input: ApplyCliConfigProfileInput,
) -> Result<CliConfigApplyResult, String> {
    api.apply_profile(input).map_err(|error| error.to_string())
}
