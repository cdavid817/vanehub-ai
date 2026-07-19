use super::dto::{Skill, SkillUpdateInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn update_skill(
    api: State<'_, SkillApi>,
    skill_id: String,
    input: SkillUpdateInput,
) -> Result<Skill, CommandError> {
    let request = mapper::update_request(skill_id, input).map_err(map_command_error)?;
    api.update(request)
        .map(mapper::record_to_dto)
        .map_err(map_command_error)
}
