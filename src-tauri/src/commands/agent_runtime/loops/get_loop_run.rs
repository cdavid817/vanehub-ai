use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_loop_run(
    api: State<'_, AgentRuntimeApi>,
    run_id: String,
) -> Result<dto::LoopRun, CommandError> {
    api.get_loop_run(&run_id)
        .map(mapper::run)
        .map_err(map_command_error)
}
