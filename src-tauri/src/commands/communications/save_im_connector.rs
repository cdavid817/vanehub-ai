use super::dto::SaveConnectorInput;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::communications::api::{CommunicationsApi, SaveConnectorRequest};
use crate::contexts::communications::application::CommunicationsApplicationError;
use crate::contexts::communications::domain::ConnectorConfig;
use tauri::State;

#[tauri::command]
pub(crate) async fn save_im_connector(
    api: State<'_, CommunicationsApi>,
    input: SaveConnectorInput,
) -> Result<ConnectorConfig, CommandError> {
    let replacement_secret = input
        .credentials
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|_| {
            map_command_error(CommunicationsApplicationError::failure(
                "credential-payload-invalid",
            ))
        })?;
    api.save_connector(SaveConnectorRequest {
        kind: input.kind,
        enabled: input.enabled,
        display_name: input.display_name,
        public_config: input.public_config,
        replacement_secret,
    })
    .await
    .map_err(map_command_error)
}
