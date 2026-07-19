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
) -> Result<OpenSessionFolderResult, CommandError> {
    let root = workspaces
        .resolve_session_root(&session_id)
        .map_err(map_command_error)?
        .ok_or_else(|| CommandError::validation("Session has no available local folder."))?;
    api.open_session_folder(&session_id, Path::new(&root), opener_id)
        .map_err(map_command_error)
}
