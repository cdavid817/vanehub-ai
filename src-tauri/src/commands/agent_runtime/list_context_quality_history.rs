use super::{context_quality_mapper, dto};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_context_quality_history(
    api: State<'_, AgentRuntimeApi>,
    input: dto::ContextQualityHistoryQuery,
) -> Result<dto::ContextQualityHistoryPage, CommandError> {
    api.list_context_quality_history(input.range_days, input.cursor.as_deref(), input.limit)
        .map(context_quality_mapper::history_to_dto)
        .map_err(map_command_error)
}
