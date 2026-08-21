use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn recover_session(
    api: State<'_, AgentRuntimeApi>,
    session_id: String,
) -> Result<dto::SessionRecoveryResult, CommandError> {
    api.recover_session(&session_id)
        .map(mapper::session_recovery_to_dto)
        .map_err(map_command_error)
}
