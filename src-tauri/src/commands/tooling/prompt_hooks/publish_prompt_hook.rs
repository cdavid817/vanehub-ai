use super::dto::{PromptHookVersion, PublishPromptHookInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use tauri::State;

#[tauri::command]
pub(crate) fn publish_prompt_hook(
    api: State<'_, PromptHookApi>,
    input: PublishPromptHookInput,
) -> Result<PromptHookVersion, CommandError> {
    let request = mapper::publish_request(input).map_err(map_command_error)?;
    api.publish(request)
        .map(mapper::version_to_dto)
        .map_err(map_command_error)
}
