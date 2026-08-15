use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::{AgentRuntimeApi, ToolApprovalDecision};
use tauri::State;

/// Delivers a user's answer to a blocked `ask_user_question` call
/// (`add-agent-user-question`).
///
/// Deliberately not routed through `resolve_pending_approval`: an approval carries a permission
/// record that gets audited and may become a grant, while an answer authorizes nothing. Sharing
/// that command would create a permission decision for a question that never asked for one.
/// The *transport* is still shared — both resolve through the same blocked-tool-call channel.
///
/// Returns whether a live waiter received the answer, so the caller can tell a delivered answer
/// from one aimed at a question that already resolved or belongs to a dead generation.
#[tauri::command]
pub(crate) fn resolve_agent_question(
    api: State<'_, AgentRuntimeApi>,
    session_id: String,
    call_id: String,
    answer: String,
) -> Result<bool, CommandError> {
    api.resolve_tool_approval(
        &session_id,
        &call_id,
        ToolApprovalDecision::Answered(answer),
    )
    .map_err(map_command_error)
}
