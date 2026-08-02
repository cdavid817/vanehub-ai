use crate::contexts::tooling::cli_config::{CliConfigApi, CliConfigStatus};
use tauri::State;

#[tauri::command]
pub(crate) fn get_cli_config_status(
    api: State<'_, CliConfigApi>,
    agent_id: String,
) -> Result<CliConfigStatus, String> {
    api.inspect_status(&agent_id)
        .map_err(|error| error.to_string())
}
