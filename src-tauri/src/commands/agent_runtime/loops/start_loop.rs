use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn start_loop(
    api: State<'_, AgentRuntimeApi>,
    definition_id: String,
) -> Result<dto::StartLoopResult, CommandError> {
    let started = api.start_loop(&definition_id).map_err(map_command_error)?;
    let run = api
        .get_loop_run(&started.run_id)
        .map_err(map_command_error)?;
    Ok(dto::StartLoopResult {
        run: mapper::run(run),
        operation_id: started.operation_id,
    })
}
