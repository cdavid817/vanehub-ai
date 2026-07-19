use super::dto::{PromptHookPreview, PromptHookPreviewInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use tauri::State;

#[tauri::command]
pub(crate) fn preview_prompt_hook(
    api: State<'_, PromptHookApi>,
    input: PromptHookPreviewInput,
) -> Result<PromptHookPreview, CommandError> {
    let request = mapper::preview_request(input).map_err(map_command_error)?;
    api.preview(request)
        .map(mapper::preview_to_dto)
        .map_err(map_command_error)
}
