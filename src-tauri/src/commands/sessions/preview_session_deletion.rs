use super::background;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::{
    PreviewSessionDeletionRequest, SessionDeletionPreview, SessionsApi,
};
use tauri::State;

/// Read-only. Runs off the main thread because it probes every worktree in the selection, and a
/// probe that hangs on a slow volume must not freeze the window.
#[tauri::command]
pub(crate) async fn preview_session_deletion(
    api: State<'_, SessionsApi>,
    input: PreviewSessionDeletionRequest,
) -> Result<SessionDeletionPreview, CommandError> {
    background::preview_deletion_off_thread(api.inner().clone(), input)
        .await
        .map_err(map_command_error)
}
