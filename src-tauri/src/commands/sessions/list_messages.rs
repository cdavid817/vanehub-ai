use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_messages(
    api: State<'_, SessionsApi>,
    session_id: String,
    limit: Option<i64>,
    before_id: Option<String>,
) -> Result<Vec<dto::ChatMessage>, CommandError> {
    api.list_messages(&session_id, limit, before_id)
        .map(mapper::messages_to_dto)
        .map_err(map_command_error)
}
