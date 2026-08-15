use super::{context_quality_mapper, dto};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_context_quality_summary(
    api: State<'_, AgentRuntimeApi>,
    input: dto::ContextQualitySummaryQuery,
) -> Result<dto::ContextQualitySummary, CommandError> {
    api.context_quality_summary(input.range_days)
        .map(|summary| context_quality_mapper::summary_to_dto(input.range_days, summary))
        .map_err(map_command_error)
}
