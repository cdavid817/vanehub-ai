use super::dto::{Skill, SkillMutationInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn create_skill(
    api: State<'_, SkillApi>,
    input: SkillMutationInput,
) -> Result<Skill, CommandError> {
    let request = mapper::create_request(input).map_err(map_command_error)?;
    api.create(request)
        .map(mapper::record_to_dto)
        .map_err(map_command_error)
}
