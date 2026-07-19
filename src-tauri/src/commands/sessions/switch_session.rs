use super::{dto, events, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::{AppHandle, State};

#[tauri::command]
pub(crate) fn switch_session(
    app: AppHandle,
    api: State<'_, SessionsApi>,
    session_id: String,
) -> Result<dto::Session, CommandError> {
    let session = api
        .switch(&session_id)
        .and_then(mapper::session_to_dto)
        .map_err(map_command_error)?;
    events::emit_active_session_changed(&app, Some(&session.id));
    Ok(session)
}
