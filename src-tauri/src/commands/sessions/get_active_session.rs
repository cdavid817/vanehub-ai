use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_active_session(
    api: State<'_, SessionsApi>,
) -> Result<Option<dto::Session>, CommandError> {
    api.active()
        .and_then(|session| session.map(mapper::session_to_dto).transpose())
        .map_err(map_command_error)
}
