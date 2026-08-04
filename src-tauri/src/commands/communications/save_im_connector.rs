use super::dto::SaveConnectorInput;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::communications::api::{CommunicationsApi, SaveConnectorRequest};
use crate::contexts::communications::domain::ConnectorConfig;
use tauri::State;

#[tauri::command]
pub(crate) async fn save_im_connector(
    api: State<'_, CommunicationsApi>,
    input: SaveConnectorInput,
) -> Result<ConnectorConfig, CommandError> {
    api.save_connector(SaveConnectorRequest {
        kind: input.kind,
        enabled: input.enabled,
        display_name: input.display_name,
        public_config: input.public_config,
        credential_patch: input.credentials,
    })
    .await
    .map_err(map_command_error)
}
