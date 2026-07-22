use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::ssh_connections::api::SshConnectionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_ssh_connections(
    api: State<'_, SshConnectionsApi>,
) -> Result<Vec<dto::SshConnection>, CommandError> {
    api.list()
        .map(|connections| {
            connections
                .into_iter()
                .map(mapper::connection_to_dto)
                .collect()
        })
        .map_err(map_command_error)
}
