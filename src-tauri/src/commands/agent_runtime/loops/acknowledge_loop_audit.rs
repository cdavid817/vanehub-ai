use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn acknowledge_loop_audit(
    api: State<'_, AgentRuntimeApi>,
    challenge_id: String,
) -> Result<dto::LoopAuditAcknowledgement, CommandError> {
    api.acknowledge_loop_audit(&challenge_id)
        .map(mapper::acknowledgement)
        .map_err(map_command_error)
}
