use super::dto::ResolvePendingApprovalInput;
use super::mapper::{approval_decision, parse_scope, tool_approval_decision};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::permissions::api::PermissionsApi;
use tauri::State;

/// Orchestrates both context facades (a command adapter, not a domain, is the one place allowed
/// to hold both — design.md D8's implementation-time refinement): first delivers the decision
/// through the *existing* `AgentRuntimeApi::resolve_tool_approval` channel to actually unblock
/// the waiting generation, then tells `permissions` to finalize (audit + grant) using whether
/// that delivery reached a live generation. The `bool` -> decision-enum mapping itself lives in
/// `mapper` (not inline here) so this adapter stays a straight-line delegation with no branching
/// of its own (`tests/architecture.rs`'s zero-control-flow budget for command adapters).
#[tauri::command]
pub(crate) fn resolve_pending_approval(
    permissions: State<'_, PermissionsApi>,
    agent_runtime: State<'_, AgentRuntimeApi>,
    input: ResolvePendingApprovalInput,
) -> Result<bool, CommandError> {
    let Some(pending) = permissions.get_pending_approval(&input.request_id) else {
        return Ok(false);
    };
    let delivered = agent_runtime
        .resolve_tool_approval(
            &pending.session_id,
            &pending.call_id,
            tool_approval_decision(input.approved),
        )
        .map_err(map_command_error)?;

    permissions
        .finalize_pending_approval(
            &input.request_id,
            approval_decision(input.approved),
            parse_scope(&input.scope),
            delivered,
        )
        .map_err(map_command_error)?;
    Ok(delivered)
}
