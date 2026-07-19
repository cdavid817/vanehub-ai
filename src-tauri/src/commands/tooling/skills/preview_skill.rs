use super::dto::{SkillPreview, SkillScopeInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn preview_skill(
    api: State<'_, SkillApi>,
    skill_id: String,
    input: SkillScopeInput,
) -> Result<SkillPreview, CommandError> {
    let key = mapper::key(skill_id, input).map_err(map_command_error)?;
    api.preview(key)
        .map(mapper::preview_to_dto)
        .map_err(map_command_error)
}
