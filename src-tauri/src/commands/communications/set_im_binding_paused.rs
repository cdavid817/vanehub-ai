use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::communications::{api::CommunicationsApi, domain::SessionBinding};
use tauri::State;

#[tauri::command]
pub(crate) fn set_im_binding_paused(
    api: State<'_, CommunicationsApi>,
    session_id: String,
    paused: bool,
) -> Result<SessionBinding, CommandError> {
    api.set_binding_paused(&session_id, paused)
        .map_err(map_command_error)
}
