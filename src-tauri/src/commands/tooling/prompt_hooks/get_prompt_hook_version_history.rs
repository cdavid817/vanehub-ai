use super::dto::PromptHookVersionHistory;
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_prompt_hook_version_history(
    api: State<'_, PromptHookApi>,
    hook_id: String,
) -> Result<PromptHookVersionHistory, CommandError> {
    let hook_id = mapper::hook_id(hook_id).map_err(map_command_error)?;
    api.version_history(hook_id)
        .map(mapper::history_to_dto)
        .map_err(map_command_error)
}
