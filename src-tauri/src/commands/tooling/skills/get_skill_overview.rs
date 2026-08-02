use super::dto::{SkillOverview, SkillScopeInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_skill_overview(
    api: State<'_, SkillApi>,
    input: SkillScopeInput,
) -> Result<SkillOverview, CommandError> {
    let query = mapper::scope_query(input).map_err(map_command_error)?;
    api.overview(query)
        .map(mapper::overview_to_dto)
        .map_err(map_command_error)
}
