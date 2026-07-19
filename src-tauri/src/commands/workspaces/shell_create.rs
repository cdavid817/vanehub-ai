use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) fn shell_create(
    api: State<'_, WorkspaceApi>,
    input: dto::CreateShellInput,
) -> Result<dto::ShellSession, CommandError> {
    let request = mapper::create_shell_request_from_dto(input);
    api.create_shell(&request)
        .map(mapper::shell_session_to_dto)
        .map_err(map_command_error)
}
