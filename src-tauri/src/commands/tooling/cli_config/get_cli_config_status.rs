use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::cli_config::{CliConfigApi, CliConfigStatus};
use tauri::State;

#[tauri::command]
pub(crate) fn get_cli_config_status(
    api: State<'_, CliConfigApi>,
    agent_id: String,
) -> Result<CliConfigStatus, CommandError> {
    api.inspect_status(&agent_id).map_err(map_command_error)
}
