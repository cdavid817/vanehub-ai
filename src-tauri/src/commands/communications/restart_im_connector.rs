use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::communications::api::CommunicationsApi;
use crate::contexts::communications::domain::ConnectorKind;
use tauri::State;

#[tauri::command]
pub(crate) async fn restart_im_connector(
    api: State<'_, CommunicationsApi>,
    kind: ConnectorKind,
) -> Result<(), CommandError> {
    api.restart_connector(kind).await.map_err(map_command_error)
}
