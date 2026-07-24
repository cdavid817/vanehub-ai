use super::dto::{PromptHookDraft, SavePromptHookDraftInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use tauri::State;

#[tauri::command]
pub(crate) fn save_prompt_hook_draft(
    api: State<'_, PromptHookApi>,
    input: SavePromptHookDraftInput,
) -> Result<PromptHookDraft, CommandError> {
    let request = mapper::save_draft_request(input).map_err(map_command_error)?;
    api.save_draft(request)
        .map(mapper::draft_to_dto)
        .map_err(map_command_error)
}
