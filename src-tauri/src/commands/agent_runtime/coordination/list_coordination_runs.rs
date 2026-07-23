use super::super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_coordination_runs(
    api: State<'_, AgentRuntimeApi>,
) -> Result<Vec<dto::CoordinationRun>, CommandError> {
    api.list_coordination_runs()
        .map(|runs| {
            runs.into_iter()
                .map(mapper::coordination_run_to_dto)
                .collect()
        })
        .map_err(map_command_error)
}
