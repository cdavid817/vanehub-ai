use super::dto::{Skill, SkillScopeInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn bind_skill_to_cli_agent(
    api: State<'_, SkillApi>,
    skill_id: String,
    input: SkillScopeInput,
    agent_id: String,
) -> Result<Skill, CommandError> {
    let key = mapper::key(skill_id, input).map_err(map_command_error)?;
    api.bind_to_cli_agent(key, agent_id)
        .map(mapper::record_to_dto)
        .map_err(map_command_error)
}
