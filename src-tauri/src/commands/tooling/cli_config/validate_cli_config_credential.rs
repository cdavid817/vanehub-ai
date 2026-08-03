use crate::contexts::tooling::cli_config::{CliConfigApi, ValidateCliConfigCredentialInput};
use crate::platform::network::ProviderCredentialValidationResult;
use tauri::State;

#[tauri::command]
pub(crate) async fn validate_cli_config_credential(
    api: State<'_, CliConfigApi>,
    input: ValidateCliConfigCredentialInput,
) -> Result<ProviderCredentialValidationResult, String> {
    api.validate_credential(input)
        .await
        .map_err(|error| error.to_string())
}
