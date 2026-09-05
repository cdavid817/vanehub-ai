use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::{SessionDeletionOperation, SessionsApi};
use tauri::State;

#[tauri::command]
pub(crate) fn get_session_deletion_operation(
    api: State<'_, SessionsApi>,
    operation_id: String,
) -> Result<SessionDeletionOperation, CommandError> {
    api.deletion_operation(&operation_id)
        .map_err(map_command_error)
}
