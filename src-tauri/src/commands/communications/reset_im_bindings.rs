use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::communications::api::CommunicationsApi;
use crate::contexts::communications::domain::ConnectorKind;
use tauri::State;

#[tauri::command]
pub(crate) fn reset_im_bindings(
    api: State<'_, CommunicationsApi>,
    kind: Option<ConnectorKind>,
) -> Result<(), CommandError> {
    api.reset_bindings(kind).map_err(map_command_error)
}
