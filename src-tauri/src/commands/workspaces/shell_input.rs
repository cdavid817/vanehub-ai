use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) fn shell_input(
    api: State<'_, WorkspaceApi>,
    shell_id: String,
    content: String,
) -> Result<(), CommandError> {
    api.write_shell_input(&shell_id, &content)
        .map_err(map_command_error)
}
