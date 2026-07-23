use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn create_loop_definition(
    api: State<'_, AgentRuntimeApi>,
    input: dto::SaveLoopDefinitionInput,
) -> Result<dto::LoopDefinition, CommandError> {
    api.create_loop_definition(mapper::save_request(input)?)
        .map(mapper::definition)
        .map_err(map_command_error)
}
