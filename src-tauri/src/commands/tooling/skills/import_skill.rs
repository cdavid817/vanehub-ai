use super::dto::{Skill, SkillImportInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn import_skill(
    api: State<'_, SkillApi>,
    input: SkillImportInput,
) -> Result<Skill, CommandError> {
    let request = mapper::import_request(input).map_err(map_command_error)?;
    api.import(request)
        .map(mapper::record_to_dto)
        .map_err(map_command_error)
}
