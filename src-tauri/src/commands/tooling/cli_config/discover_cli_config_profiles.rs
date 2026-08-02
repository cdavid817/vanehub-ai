use crate::contexts::tooling::cli_config::{CliConfigApi, CliConfigDiscoveryResult};
use tauri::State;

#[tauri::command]
pub(crate) fn discover_cli_config_profiles(
    api: State<'_, CliConfigApi>,
    agent_id: String,
) -> Result<CliConfigDiscoveryResult, String> {
    api.discover_profiles(&agent_id)
        .map_err(|error| error.to_string())
}
