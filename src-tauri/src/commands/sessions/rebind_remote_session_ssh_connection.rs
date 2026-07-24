use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::commands::sessions::dto;
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn rebind_remote_session_ssh_connection(
    api: State<'_, SessionsApi>,
    session_id: String,
    connection_id: String,
) -> Result<dto::Session, CommandError> {
    api.rebind_remote_ssh_connection(&session_id, &connection_id)
        .and_then(mapper::session_to_dto)
        .map_err(map_command_error)
}
