use super::dto::{SkillScopeInput, SkillSyncResult};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn sync_skill_drift(
    api: State<'_, SkillApi>,
    input: SkillScopeInput,
) -> Result<SkillSyncResult, CommandError> {
    let query = mapper::scope_query(input).map_err(map_command_error)?;
    api.sync_drift(query)
        .map(mapper::sync_to_dto)
        .map_err(map_command_error)
}
