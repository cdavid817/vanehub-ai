use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::{DesktopSettingsApi, FolderOpenerId, OpenSessionFolderResult};
use crate::contexts::workspaces::api::WorkspaceApi;
use std::path::Path;
use tauri::State;

#[tauri::command]
pub(crate) fn open_session_folder(
    api: State<'_, DesktopSettingsApi>,
    workspaces: State<'_, WorkspaceApi>,
    session_id: String,
    opener_id: FolderOpenerId,
    relative_path: Option<String>,
) -> Result<OpenSessionFolderResult, CommandError> {
    // Resolved by the workspaces context, not here. A file manager opens whatever absolute path it
    // is handed, so a relative directory has to be checked against the workspace it claims to be
    // inside — and that check is I/O against a canonical root, which is not a command's work.
    let target = workspaces
        .resolve_session_directory(&session_id, relative_path.as_deref().unwrap_or_default())
        .map_err(map_command_error)?
        .ok_or_else(|| CommandError::validation("Session has no available local folder."))?;
    api.open_session_folder(&session_id, Path::new(&target), opener_id)
        .map_err(map_command_error)
}
