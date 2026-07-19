use super::dto::PromptHookTraceSummary;
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_prompt_hook_traces(
    api: State<'_, PromptHookApi>,
    limit: Option<i64>,
) -> Result<Vec<PromptHookTraceSummary>, CommandError> {
    api.list_traces(limit.unwrap_or(25))
        .map(mapper::traces_to_dto)
        .map_err(map_command_error)
}
