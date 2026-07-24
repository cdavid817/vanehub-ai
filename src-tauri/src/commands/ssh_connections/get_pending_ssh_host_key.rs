use super::{dto, mapper};
use crate::contexts::ssh_connections::api::SshConnectionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_pending_ssh_host_key(
    connection_id: String,
    api: State<'_, SshConnectionsApi>,
) -> Option<dto::SshHostKeyChallenge> {
    api.pending_host_key(&connection_id)
        .map(mapper::host_key_challenge_to_dto)
}
