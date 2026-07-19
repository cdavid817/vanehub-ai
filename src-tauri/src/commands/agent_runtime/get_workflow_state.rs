use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_workflow_state(
    api: State<'_, AgentRuntimeApi>,
) -> Result<dto::WorkflowState, CommandError> {
    api.workflow()
        .map(mapper::workflow_to_dto)
        .map_err(map_command_error)
}
