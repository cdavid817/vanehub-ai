use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn select_agent(
    api: State<'_, AgentRuntimeApi>,
    agent_id: String,
    interaction_mode: dto::InteractionMode,
) -> Result<dto::WorkflowState, CommandError> {
    api.select_agent(
        &agent_id,
        mapper::interaction_mode_from_dto(interaction_mode),
    )
    .map(mapper::workflow_to_dto)
    .map_err(map_command_error)
}
