use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

/// Delivers the user's decision on a blocked `exit_plan_mode` call
/// (`add-agent-plan-exit-request`).
///
/// Deliberately not routed through `resolve_pending_approval`, for the same reason
/// `resolve_agent_question` is not: that command writes a permission record which may harden into
/// a standing grant. This decision authorizes neither an action nor a resource -- only this
/// session's move out of plan mode, once. A grant here would read as "always allowed to leave plan
/// mode," which is the opposite of what plan mode is for. The *transport* is still shared: both
/// resolve through the same blocked-tool-call channel.
///
/// Returns whether a live waiter received the decision, so the caller can tell a delivered
/// decision from one aimed at a request that already resolved or belongs to a dead generation --
/// which is what gates the session actually changing mode.
#[tauri::command]
pub(crate) fn resolve_plan_exit(
    api: State<'_, AgentRuntimeApi>,
    session_id: String,
    call_id: String,
    approved: bool,
) -> Result<bool, CommandError> {
    api.resolve_plan_exit(&session_id, &call_id, approved)
        .map_err(map_command_error)
}
