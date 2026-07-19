use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn stop_generation(
    api: State<'_, AgentRuntimeApi>,
    session_id: String,
) -> Result<(), CommandError> {
    api.stop_generation(&session_id)
        .map(|_| ())
        .map_err(map_command_error)
}
