use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use tauri::State;

#[tauri::command]
pub(crate) fn delete_prompt_hook(
    api: State<'_, PromptHookApi>,
    hook_id: String,
) -> Result<(), CommandError> {
    let hook_id = mapper::hook_id(hook_id).map_err(map_command_error)?;
    api.delete(hook_id).map_err(map_command_error)
}
