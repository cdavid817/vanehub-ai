use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::communications::{api::CommunicationsApi, domain::ConnectorKind};
use tauri::State;

#[tauri::command]
pub(crate) fn cancel_im_pairing(
    api: State<'_, CommunicationsApi>,
    session_id: String,
    connector: ConnectorKind,
) -> Result<bool, CommandError> {
    api.cancel_pairing(&session_id, connector)
        .map_err(map_command_error)
}
