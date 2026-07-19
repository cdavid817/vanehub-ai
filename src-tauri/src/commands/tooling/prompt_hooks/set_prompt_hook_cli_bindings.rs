use super::dto::PromptHook;
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use tauri::State;

#[tauri::command]
pub(crate) fn set_prompt_hook_cli_bindings(
    api: State<'_, PromptHookApi>,
    hook_id: String,
    agent_ids: Vec<String>,
) -> Result<PromptHook, CommandError> {
    let hook_id = mapper::hook_id(hook_id).map_err(map_command_error)?;
    let bindings = mapper::bindings(agent_ids).map_err(map_command_error)?;
    api.set_bindings(hook_id, bindings)
        .map(mapper::record_to_dto)
        .map_err(map_command_error)
}
