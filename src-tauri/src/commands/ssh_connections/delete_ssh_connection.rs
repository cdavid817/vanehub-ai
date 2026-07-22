use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::ssh_connections::api::SshConnectionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn delete_ssh_connection(
    api: State<'_, SshConnectionsApi>,
    connection_id: String,
) -> Result<(), CommandError> {
    api.delete(&connection_id).map_err(map_command_error)
}
