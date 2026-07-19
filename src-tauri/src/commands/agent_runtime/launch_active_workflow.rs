use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn launch_active_workflow(
    api: State<'_, AgentRuntimeApi>,
) -> Result<dto::LaunchResult, CommandError> {
    api.launch_active_workflow()
        .map(mapper::launch_to_dto)
        .map_err(map_command_error)
}
