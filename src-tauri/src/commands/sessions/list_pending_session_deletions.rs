use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::{SessionDeletionOperation, SessionsApi};
use tauri::State;

#[tauri::command]
pub(crate) fn list_pending_session_deletions(
    api: State<'_, SessionsApi>,
) -> Result<Vec<SessionDeletionOperation>, CommandError> {
    api.list_pending_deletions().map_err(map_command_error)
}
