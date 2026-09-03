use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::communications::api::CommunicationsApi;
use crate::contexts::communications::domain::{ConnectorKind, SessionConnectorAccess};
use tauri::State;

#[tauri::command]
pub(crate) fn set_im_session_access(
    api: State<'_, CommunicationsApi>,
    session_id: String,
    connector: ConnectorKind,
    enabled: bool,
) -> Result<SessionConnectorAccess, CommandError> {
    api.set_session_access(&session_id, connector, enabled)
        .map_err(map_command_error)
}
