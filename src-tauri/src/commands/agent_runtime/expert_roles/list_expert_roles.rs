use crate::commands::agent_runtime::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_expert_roles(
    api: State<'_, AgentRuntimeApi>,
) -> Result<Vec<dto::ExpertRole>, CommandError> {
    api.list_expert_roles()
        .map(|roles| roles.into_iter().map(mapper::expert_role_to_dto).collect())
        .map_err(map_command_error)
}
