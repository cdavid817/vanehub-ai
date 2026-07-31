use super::dto::SkillScopeInput;
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_skill_api_agent_bindings(
    api: State<'_, SkillApi>,
    skill_id: String,
    input: SkillScopeInput,
) -> Result<Vec<String>, CommandError> {
    let key = mapper::key(skill_id, input).map_err(map_command_error)?;
    api.list_api_agent_bindings(key).map_err(map_command_error)
}
