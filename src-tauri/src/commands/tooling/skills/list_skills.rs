use super::dto::{SkillListResult, SkillScopeInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_skills(
    api: State<'_, SkillApi>,
    input: SkillScopeInput,
) -> Result<SkillListResult, CommandError> {
    let query = mapper::scope_query(input).map_err(map_command_error)?;
    api.list(query)
        .map(mapper::list_to_dto)
        .map_err(map_command_error)
}
