use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::communications::api::CommunicationsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn remove_im_session_binding(
    api: State<'_, CommunicationsApi>,
    session_id: String,
) -> Result<bool, CommandError> {
    api.remove_binding(&session_id).map_err(map_command_error)
}
