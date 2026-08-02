use crate::contexts::tooling::cli_config::{CliConfigApi, CliConfigProfile};
use tauri::State;

#[tauri::command]
pub(crate) fn list_cli_config_profiles(
    api: State<'_, CliConfigApi>,
    agent_id: String,
) -> Result<Vec<CliConfigProfile>, String> {
    api.list_profiles(&agent_id)
        .map_err(|error| error.to_string())
}
