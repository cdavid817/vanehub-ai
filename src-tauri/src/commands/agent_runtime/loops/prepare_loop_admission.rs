use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn prepare_loop_admission(
    api: State<'_, AgentRuntimeApi>,
    input: dto::PrepareLoopAdmissionInput,
) -> Result<dto::LoopAdmission, CommandError> {
    let request = mapper::admission_request(input)?;
    api.prepare_loop_admission(request)
        .map(mapper::admission)
        .map_err(map_command_error)
}
