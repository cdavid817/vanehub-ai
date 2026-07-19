use super::dto::{PromptAssemblyPreviewInput, PromptHookPreview};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use tauri::State;

#[tauri::command]
pub(crate) fn preview_prompt_assembly(
    api: State<'_, PromptHookApi>,
    input: PromptAssemblyPreviewInput,
) -> Result<PromptHookPreview, CommandError> {
    let result = api
        .effective_prompt(&input.agent_id, None, &input.sample_input)
        .map_err(map_command_error)?;
    Ok(mapper::assembly_to_dto(input.agent_id, result))
}
