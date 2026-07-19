use super::dto::{PromptHook, PromptHookUpdateInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use tauri::State;

#[tauri::command]
pub(crate) fn update_prompt_hook(
    api: State<'_, PromptHookApi>,
    hook_id: String,
    input: PromptHookUpdateInput,
) -> Result<PromptHook, CommandError> {
    let request = mapper::update_request(hook_id, input).map_err(map_command_error)?;
    api.update(request)
        .map(mapper::record_to_dto)
        .map_err(map_command_error)
}
