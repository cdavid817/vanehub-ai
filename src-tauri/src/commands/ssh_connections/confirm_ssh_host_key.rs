use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::ssh_connections::api::SshConnectionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn confirm_ssh_host_key(
    input: dto::ConfirmSshHostKeyInput,
    api: State<'_, SshConnectionsApi>,
) -> Result<dto::SshHostKeyChallenge, CommandError> {
    api.confirm_host_key(&input.connection_id, input.revision, &input.fingerprint)
        .map(mapper::host_key_challenge_to_dto)
        .map_err(map_command_error)
}
