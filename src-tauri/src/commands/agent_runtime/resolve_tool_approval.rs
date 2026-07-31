use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn resolve_tool_approval(
    api: State<'_, AgentRuntimeApi>,
    input: dto::ResolveToolApprovalInput,
) -> Result<bool, CommandError> {
    api.resolve_tool_approval(
        &input.session_id,
        &input.call_id,
        mapper::tool_approval_decision(input.approved),
    )
    .map_err(map_command_error)
}
