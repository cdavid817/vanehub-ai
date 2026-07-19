use super::dto::{Skill, SkillScopeInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn set_skill_agent_bindings(
    api: State<'_, SkillApi>,
    skill_id: String,
    input: SkillScopeInput,
    agent_ids: Vec<String>,
) -> Result<Skill, CommandError> {
    let key = mapper::key(skill_id, input).map_err(map_command_error)?;
    api.set_bindings(key, agent_ids)
        .map(mapper::record_to_dto)
        .map_err(map_command_error)
}
