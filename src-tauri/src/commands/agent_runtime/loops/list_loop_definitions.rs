use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_loop_definitions(
    api: State<'_, AgentRuntimeApi>,
) -> Result<Vec<dto::LoopDefinition>, CommandError> {
    api.list_loop_definitions()
        .map(|values| values.into_iter().map(mapper::definition).collect())
        .map_err(map_command_error)
}
