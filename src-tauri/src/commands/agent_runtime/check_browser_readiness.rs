use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn check_browser_readiness(
    api: State<'_, AgentRuntimeApi>,
    agent_id: String,
) -> Result<dto::ReadinessStatus, CommandError> {
    api.browser_readiness(&agent_id)
        .map(mapper::readiness_to_dto)
        .map_err(map_command_error)
}
