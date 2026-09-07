use super::background;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::{
    RetrySessionDeletionRequest, SessionDeletionHandle, SessionsApi,
};
use tauri::State;

#[tauri::command]
pub(crate) fn retry_session_deletion(
    api: State<'_, SessionsApi>,
    input: RetrySessionDeletionRequest,
) -> Result<SessionDeletionHandle, CommandError> {
    let handle = api.retry_deletion(input).map_err(map_command_error)?;
    background::spawn_deletion_unless_existing(api.inner().clone(), &handle);
    Ok(handle)
}
