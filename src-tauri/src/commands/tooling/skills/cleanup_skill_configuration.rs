use super::dto::{SkillConfigurationRetentionOutcome, SkillScopeInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::{DeletionRetention, SkillApi};
use tauri::State;

/// Removes retained orphan configuration and its credentials. This is an explicit maintenance
/// action rather than an install or upgrade side effect, so a Skill that disappears during an
/// upgrade keeps its values until someone asks for them to go.
#[tauri::command]
pub(crate) fn cleanup_skill_configuration(
    api: State<'_, SkillApi>,
    skill_id: String,
    input: SkillScopeInput,
) -> Result<SkillConfigurationRetentionOutcome, CommandError> {
    let key = mapper::key(skill_id, input).map_err(map_command_error)?;
    api.apply_configuration_retention(&key, DeletionRetention::Delete)
        .map(mapper::retention_outcome_to_dto)
        .map_err(map_command_error)
}
