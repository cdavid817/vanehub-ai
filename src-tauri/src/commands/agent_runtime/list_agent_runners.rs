use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_agent_runners(
    api: State<'_, AgentRuntimeApi>,
    session_id: String,
    agent_id: String,
) -> Result<Vec<dto::RunnerDescriptor>, CommandError> {
    api.list_runners(&session_id, &agent_id)
        .map(mapper::runners_to_dto)
        .map_err(map_command_error)
}
