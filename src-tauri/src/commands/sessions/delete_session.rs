use super::events;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::sessions::api::SessionsApi;
use tauri::{AppHandle, State};

#[tauri::command]
pub(crate) fn delete_session(
    app: AppHandle,
    api: State<'_, SessionsApi>,
    agent_runtime: State<'_, AgentRuntimeApi>,
    session_id: String,
) -> Result<(), CommandError> {
    // Owned runtime resources go first: an ACP agent process, its pending approvals, and its
    // proxied terminals belong to this session and nothing else. The CLI's own history, its
    // credential store, and any worktree are untouched -- those follow their own cleanup flows.
    let _ = agent_runtime.release_managed_connections(&session_id);
    api.delete(&session_id).map_err(map_command_error)?;
    events::emit_active_session_changed(&app, None);
    Ok(())
}
