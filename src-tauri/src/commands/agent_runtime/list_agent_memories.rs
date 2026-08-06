use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_agent_memories(
    api: State<'_, AgentRuntimeApi>,
) -> Result<Vec<dto::AgentMemoryEntry>, CommandError> {
    api.list_all_memories()
        .map(mapper::agent_memories_to_dto)
        .map_err(map_command_error)
}
