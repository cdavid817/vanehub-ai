use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::cli_config::{CliConfigApi, ValidateCliConfigCredentialInput};
use crate::platform::network::ProviderCredentialValidationResult;
use tauri::State;

#[tauri::command]
pub(crate) async fn validate_cli_config_credential(
    api: State<'_, CliConfigApi>,
    input: ValidateCliConfigCredentialInput,
) -> Result<ProviderCredentialValidationResult, CommandError> {
    api.validate_credential(input)
        .await
        .map_err(map_command_error)
}
