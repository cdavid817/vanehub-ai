use super::dto::{Skill, SkillScopeInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn set_skill_enabled(
    api: State<'_, SkillApi>,
    skill_id: String,
    input: SkillScopeInput,
    enabled: bool,
) -> Result<Skill, CommandError> {
    let key = mapper::key(skill_id, input).map_err(map_command_error)?;
    api.set_enabled(key, enabled)
        .map(mapper::record_to_dto)
        .map_err(map_command_error)
}
