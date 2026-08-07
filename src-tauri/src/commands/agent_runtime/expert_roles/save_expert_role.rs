use crate::commands::agent_runtime::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn save_expert_role(
    api: State<'_, AgentRuntimeApi>,
    input: dto::SaveExpertRoleInput,
) -> Result<dto::ExpertRole, CommandError> {
    let (role_id, role_input) = mapper::save_expert_role_request(input);
    api.save_expert_role(role_id, role_input)
        .map(mapper::expert_role_to_dto)
        .map_err(map_command_error)
}
