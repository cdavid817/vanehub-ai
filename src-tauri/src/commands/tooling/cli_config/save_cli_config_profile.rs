use crate::contexts::tooling::cli_config::{
    CliConfigApi, CliConfigProfile, SaveCliConfigProfileInput,
};
use tauri::State;

#[tauri::command]
pub(crate) fn save_cli_config_profile(
    api: State<'_, CliConfigApi>,
    input: SaveCliConfigProfileInput,
) -> Result<CliConfigProfile, String> {
    api.save_profile(input).map_err(|error| error.to_string())
}
