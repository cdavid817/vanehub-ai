use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn rename_session(
    api: State<'_, SessionsApi>,
    session_id: String,
    title: String,
) -> Result<dto::Session, CommandError> {
    api.rename(&session_id, title)
        .and_then(mapper::session_to_dto)
        .map_err(map_command_error)
}
