use crate::contexts::tooling::cli_config::{CliConfigApi, CliConfigProfile};
use tauri::State;

#[tauri::command]
pub(crate) fn duplicate_cli_config_profile(
    api: State<'_, CliConfigApi>,
    agent_id: String,
    profile_id: String,
) -> Result<CliConfigProfile, String> {
    api.duplicate_profile(&agent_id, &profile_id)
        .map_err(|error| error.to_string())
}
