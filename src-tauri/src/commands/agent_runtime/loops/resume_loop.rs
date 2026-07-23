use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn resume_loop(
    api: State<'_, AgentRuntimeApi>,
    run_id: String,
) -> Result<dto::LoopRun, CommandError> {
    api.resume_loop(&run_id)
        .map(mapper::run)
        .map_err(map_command_error)
}
