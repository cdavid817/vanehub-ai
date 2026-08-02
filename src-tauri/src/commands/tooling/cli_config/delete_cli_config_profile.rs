use crate::contexts::tooling::cli_config::{CliConfigApi, DeleteCliConfigProfileInput};
use tauri::State;

#[tauri::command]
pub(crate) fn delete_cli_config_profile(
    api: State<'_, CliConfigApi>,
    input: DeleteCliConfigProfileInput,
) -> Result<(), String> {
    api.delete_profile(input).map_err(|error| error.to_string())
}
