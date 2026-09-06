use super::background;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::{
    ExecuteSessionDeletionRequest, SessionDeletionHandle, SessionsApi,
};
use tauri::State;

/// Accepts the request into the journal and returns its handle. The stop, the removal and the
/// row deletion run afterwards on the blocking pool; the handle is not a result.
#[tauri::command]
pub(crate) fn execute_session_deletion(
    api: State<'_, SessionsApi>,
    input: ExecuteSessionDeletionRequest,
) -> Result<SessionDeletionHandle, CommandError> {
    let handle = api.execute_deletion(input).map_err(map_command_error)?;
    background::spawn_deletion_unless_existing(api.inner().clone(), &handle);
    Ok(handle)
}
