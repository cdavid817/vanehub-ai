use super::dto::{PromptHookVersion, RollbackPromptHookInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use tauri::State;

#[tauri::command]
pub(crate) fn rollback_prompt_hook(
    api: State<'_, PromptHookApi>,
    input: RollbackPromptHookInput,
) -> Result<PromptHookVersion, CommandError> {
    let request = mapper::rollback_request(input).map_err(map_command_error)?;
    api.rollback(request)
        .map(mapper::version_to_dto)
        .map_err(map_command_error)
}
