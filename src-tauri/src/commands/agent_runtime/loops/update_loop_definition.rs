use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn update_loop_definition(
    api: State<'_, AgentRuntimeApi>,
    definition_id: String,
    input: dto::SaveLoopDefinitionInput,
) -> Result<dto::LoopDefinition, CommandError> {
    api.update_loop_definition(&definition_id, mapper::save_request(input)?)
        .map(mapper::definition)
        .map_err(map_command_error)
}
