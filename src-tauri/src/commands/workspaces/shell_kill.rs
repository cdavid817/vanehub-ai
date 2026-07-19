use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) fn shell_kill(
    api: State<'_, WorkspaceApi>,
    shell_id: String,
) -> Result<(), CommandError> {
    api.kill_shell(&shell_id).map_err(map_command_error)
}
