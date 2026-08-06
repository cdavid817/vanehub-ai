use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn delete_expert_role(
    api: State<'_, AgentRuntimeApi>,
    role_id: String,
) -> Result<(), CommandError> {
    api.delete_expert_role(&role_id).map_err(map_command_error)
}
