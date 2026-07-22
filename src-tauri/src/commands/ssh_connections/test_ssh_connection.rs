use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::ssh_connections::api::SshConnectionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn test_ssh_connection(
    api: State<'_, SshConnectionsApi>,
    connection_id: String,
) -> Result<dto::SshConnectionTestResult, CommandError> {
    api.test(&connection_id)
        .map(mapper::test_result_to_dto)
        .map_err(map_command_error)
}
