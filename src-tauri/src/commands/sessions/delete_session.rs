use super::background;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::{AppHandle, State};

/// The keep-only path. Kept for internal callers; every visible entry point uses the confirmed
/// preview/execute commands. Blocking, because it waits for the session to actually be gone.
#[tauri::command]
pub(crate) async fn delete_session(
    app: AppHandle,
    api: State<'_, SessionsApi>,
    session_id: String,
) -> Result<(), CommandError> {
    background::delete_session_off_thread(app, api.inner().clone(), session_id)
        .await
        .map_err(map_command_error)
}
