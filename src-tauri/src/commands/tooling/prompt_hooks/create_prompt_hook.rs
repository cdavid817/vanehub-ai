use super::dto::{PromptHook, PromptHookMutationInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use tauri::State;

#[tauri::command]
pub(crate) fn create_prompt_hook(
    api: State<'_, PromptHookApi>,
    input: PromptHookMutationInput,
) -> Result<PromptHook, CommandError> {
    let request = mapper::create_request(input).map_err(map_command_error)?;
    api.create(request)
        .map(mapper::record_to_dto)
        .map_err(map_command_error)
}
