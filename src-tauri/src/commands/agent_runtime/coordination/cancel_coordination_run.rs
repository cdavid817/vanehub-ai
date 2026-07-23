use super::super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn cancel_coordination_run(
    api: State<'_, AgentRuntimeApi>,
    run_id: String,
) -> Result<dto::CoordinationRun, CommandError> {
    api.cancel_coordination_run(&run_id)
        .map(mapper::coordination_run_to_dto)
        .map_err(map_command_error)
}
