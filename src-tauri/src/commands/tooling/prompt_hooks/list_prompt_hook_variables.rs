use super::dto::PromptHookVariableDefinition;
use super::mapper;
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_prompt_hook_variables(
    api: State<'_, PromptHookApi>,
) -> Vec<PromptHookVariableDefinition> {
    mapper::variables_to_dto(api.list_variables())
}
