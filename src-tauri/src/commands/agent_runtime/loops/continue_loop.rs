use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::{AgentRuntimeApi, ContinueLoopRequest};
use tauri::State;

#[tauri::command]
pub(crate) fn continue_loop(
    api: State<'_, AgentRuntimeApi>,
    input: dto::ContinueLoopInput,
) -> Result<dto::LoopRun, CommandError> {
    api.continue_loop(ContinueLoopRequest {
        run_id: input.run_id,
        feedback: input.feedback,
    })
    .map(mapper::run)
    .map_err(map_command_error)
}
