use super::dto::SkillScopeInput;
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn bind_skill_to_api_agent(
    api: State<'_, SkillApi>,
    skill_id: String,
    input: SkillScopeInput,
    agent_id: String,
) -> Result<(), CommandError> {
    let key = mapper::key(skill_id, input).map_err(map_command_error)?;
    api.bind_to_api_agent(key, agent_id)
        .map_err(map_command_error)
}
