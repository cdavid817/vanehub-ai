use super::dto::PromptHook;
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use tauri::State;

#[tauri::command]
pub(crate) fn set_prompt_hook_enabled(
    api: State<'_, PromptHookApi>,
    hook_id: String,
    enabled: bool,
) -> Result<PromptHook, CommandError> {
    let hook_id = mapper::hook_id(hook_id).map_err(map_command_error)?;
    api.set_enabled(hook_id, enabled)
        .map(mapper::record_to_dto)
        .map_err(map_command_error)
}
