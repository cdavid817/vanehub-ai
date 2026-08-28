use super::{dto::SessionBindingView, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::communications::api::CommunicationsApi;
use crate::contexts::communications::domain::ConnectorKind;
use tauri::State;

#[tauri::command]
pub(crate) fn get_im_session_binding(
    api: State<'_, CommunicationsApi>,
    session_id: String,
    connector: Option<ConnectorKind>,
) -> Result<SessionBindingView, CommandError> {
    api.session_binding(&session_id, connector.unwrap_or(ConnectorKind::Feishu))
        .map(mapper::binding)
        .map_err(map_command_error)
}
