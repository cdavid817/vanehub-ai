use super::events;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::{AppHandle, State};

#[tauri::command]
pub(crate) fn delete_session(
    app: AppHandle,
    api: State<'_, SessionsApi>,
    session_id: String,
) -> Result<(), CommandError> {
    api.delete(&session_id).map_err(map_command_error)?;
    events::emit_active_session_changed(&app, None);
    Ok(())
}
