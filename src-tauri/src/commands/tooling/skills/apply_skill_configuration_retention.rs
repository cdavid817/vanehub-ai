use super::dto::{
    SkillConfigurationRetention, SkillConfigurationRetentionOutcome, SkillScopeInput,
};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

/// Records the retain-or-delete decision taken when a configured Skill is deleted. Retaining keeps
/// the rows recoverable under the same Skill identity; deleting also attempts credential cleanup
/// and reports what it could not verify.
#[tauri::command]
pub(crate) fn apply_skill_configuration_retention(
    api: State<'_, SkillApi>,
    skill_id: String,
    input: SkillScopeInput,
    retention: SkillConfigurationRetention,
) -> Result<SkillConfigurationRetentionOutcome, CommandError> {
    let key = mapper::key(skill_id, input).map_err(map_command_error)?;
    api.apply_configuration_retention(&key, mapper::retention_to_domain(retention))
        .map(mapper::retention_outcome_to_dto)
        .map_err(map_command_error)
}
