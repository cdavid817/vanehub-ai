use crate::contexts::tooling::cli_config::{
    CliConfigApi, ImportDiscoveredCliConfigInput, ImportDiscoveredCliConfigResult,
};
use tauri::State;

#[tauri::command]
pub(crate) fn import_discovered_cli_config_profiles(
    api: State<'_, CliConfigApi>,
    input: ImportDiscoveredCliConfigInput,
) -> Result<ImportDiscoveredCliConfigResult, String> {
    api.import_discovered_profiles(input)
        .map_err(|error| error.to_string())
}
