use super::{dto::PairingStartView, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::communications::{api::CommunicationsApi, domain::ConnectorKind};
use tauri::State;

#[tauri::command]
pub(crate) async fn begin_im_pairing(
    api: State<'_, CommunicationsApi>,
    session_id: String,
    connector: ConnectorKind,
    replace_existing: bool,
) -> Result<PairingStartView, CommandError> {
    api.begin_pairing(&session_id, connector, replace_existing)
        .await
        .map(mapper::pairing)
        .map_err(map_command_error)
}
