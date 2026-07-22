use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::ssh_connections::api::SshConnectionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn create_ssh_connection(
    api: State<'_, SshConnectionsApi>,
    input: dto::SaveSshConnectionInput,
) -> Result<dto::SshConnection, CommandError> {
    let mutation = mapper::mutation_from_dto(input).map_err(map_command_error)?;
    api.create(mutation)
        .map(mapper::connection_to_dto)
        .map_err(map_command_error)
}
