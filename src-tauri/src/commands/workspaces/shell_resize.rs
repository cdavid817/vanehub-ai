use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) fn shell_resize(
    api: State<'_, WorkspaceApi>,
    input: dto::ResizeShellInput,
) -> Result<(), CommandError> {
    let request = mapper::resize_shell_request_from_dto(input);
    api.resize_shell(&request).map_err(map_command_error)
}
