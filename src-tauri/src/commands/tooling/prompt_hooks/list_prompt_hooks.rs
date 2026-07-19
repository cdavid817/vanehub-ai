use super::dto::PromptHookListResult;
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_prompt_hooks(
    api: State<'_, PromptHookApi>,
) -> Result<PromptHookListResult, CommandError> {
    api.list()
        .map(mapper::list_to_dto)
        .map_err(map_command_error)
}
