use super::dto::Skill;
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::{SkillApi, SkillError, SkillId};
use tauri::State;

#[tauri::command]
pub(crate) fn restore_builtin_skill(
    api: State<'_, SkillApi>,
    skill_id: String,
) -> Result<Skill, CommandError> {
    let skill_id = SkillId::parse(skill_id)
        .map_err(SkillError::from)
        .map_err(map_command_error)?;
    api.restore_builtin(skill_id)
        .map(mapper::record_to_dto)
        .map_err(map_command_error)
}
