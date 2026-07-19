use super::{dto, events, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::{AppHandle, State};

#[tauri::command]
pub(crate) fn archive_session(
    app: AppHandle,
    api: State<'_, SessionsApi>,
    session_id: String,
) -> Result<dto::Session, CommandError> {
    let session = api
        .set_archived(&session_id, true)
        .and_then(mapper::session_to_dto)
        .map_err(map_command_error)?;
    events::emit_active_session_changed(&app, None);
    Ok(session)
}
