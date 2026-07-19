use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::communications::api::CommunicationsApi;
use crate::contexts::communications::domain::ConnectorKind;
use tauri::State;

#[tauri::command]
pub(crate) async fn set_im_connector_enabled(
    api: State<'_, CommunicationsApi>,
    kind: ConnectorKind,
    enabled: bool,
) -> Result<(), CommandError> {
    api.set_connector_enabled(kind, enabled)
        .await
        .map_err(map_command_error)
}
