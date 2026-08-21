use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) async fn check_loop_readiness(
    api: State<'_, AgentRuntimeApi>,
    definition_id: String,
) -> Result<dto::LoopReadinessReport, CommandError> {
    api.check_loop_readiness_blocking(definition_id)
        .await
        .map(mapper::readiness)
        .map_err(map_command_error)
}
