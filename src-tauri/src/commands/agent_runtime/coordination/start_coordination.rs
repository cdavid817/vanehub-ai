use super::super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn start_coordination(
    api: State<'_, AgentRuntimeApi>,
    input: dto::StartCoordinationInput,
) -> Result<dto::StartCoordinationResult, CommandError> {
    api.start_coordination(mapper::start_coordination_request(input))
        .map(|result| dto::StartCoordinationResult {
            run_id: result.run_id,
            operation_id: result.operation_id,
        })
        .map_err(map_command_error)
}
