use crate::contexts::tooling::cli_config::{
    CliConfigApi, CliConfigProfile, ImportCliConfigProfileInput,
};
use tauri::State;

#[tauri::command]
pub(crate) fn import_cli_config_profile(
    api: State<'_, CliConfigApi>,
    input: ImportCliConfigProfileInput,
) -> Result<CliConfigProfile, String> {
    api.import_current(input).map_err(|error| error.to_string())
}
