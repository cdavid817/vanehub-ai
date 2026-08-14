use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::communications::{api::CommunicationsApi, domain::SessionBinding};
use tauri::State;

#[tauri::command]
pub(crate) fn set_im_completion_notifications(
    api: State<'_, CommunicationsApi>,
    session_id: String,
    enabled: bool,
) -> Result<SessionBinding, CommandError> {
    api.set_completion_notifications(&session_id, enabled)
        .map_err(map_command_error)
}
