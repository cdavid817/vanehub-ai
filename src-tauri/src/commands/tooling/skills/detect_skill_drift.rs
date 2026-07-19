use super::dto::{SkillDriftReport, SkillScopeInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn detect_skill_drift(
    api: State<'_, SkillApi>,
    input: SkillScopeInput,
) -> Result<SkillDriftReport, CommandError> {
    let query = mapper::scope_query(input).map_err(map_command_error)?;
    api.detect_drift(query)
        .map(mapper::drift_to_dto)
        .map_err(map_command_error)
}
