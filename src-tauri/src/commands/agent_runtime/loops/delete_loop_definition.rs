use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn delete_loop_definition(
    api: State<'_, AgentRuntimeApi>,
    definition_id: String,
) -> Result<(), CommandError> {
    api.delete_loop_definition(&definition_id)
        .map_err(map_command_error)
}
