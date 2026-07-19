use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) fn shell_cd(api: State<'_, WorkspaceApi>, shell_id: String) -> Result<(), CommandError> {
    api.reset_shell_directory(&shell_id)
        .map_err(map_command_error)
}
