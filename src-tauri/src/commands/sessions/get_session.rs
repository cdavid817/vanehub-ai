use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::{SessionsApi, SessionsError};
use tauri::State;

#[tauri::command]
pub(crate) fn get_session(
    api: State<'_, SessionsApi>,
    session_id: String,
) -> Result<dto::Session, CommandError> {
    api.find(&session_id)
        .and_then(|session| {
            session.ok_or_else(|| SessionsError::SessionNotFound(session_id.clone()))
        })
        .and_then(mapper::session_to_dto)
        .map_err(map_command_error)
}
