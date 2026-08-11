use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::cli_config::{CliConfigApi, CliConfigProfile};
use tauri::State;

#[tauri::command]
pub(crate) fn duplicate_cli_config_profile(
    api: State<'_, CliConfigApi>,
    agent_id: String,
    profile_id: String,
) -> Result<CliConfigProfile, CommandError> {
    api.duplicate_profile(&agent_id, &profile_id)
        .map_err(map_command_error)
}
