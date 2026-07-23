use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_loop_runs(
    api: State<'_, AgentRuntimeApi>,
    definition_id: Option<String>,
) -> Result<Vec<dto::LoopRun>, CommandError> {
    api.list_loop_runs(definition_id.as_deref())
        .map(|values| values.into_iter().map(mapper::run).collect())
        .map_err(map_command_error)
}
