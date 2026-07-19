use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) fn select_project_directory(
    api: State<'_, WorkspaceApi>,
) -> Result<Option<String>, CommandError> {
    api.select_project_directory().map_err(map_command_error)
}
