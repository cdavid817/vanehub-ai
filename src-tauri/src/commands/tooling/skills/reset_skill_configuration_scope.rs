use super::dto::{SkillConfigurationResolutionOutcome, SkillConfigurationScopeInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

/// Deletes the scope's whole record. Absence is what makes the effective value fall through to the
/// next scope, so this is not the same as saving an empty record.
#[tauri::command]
pub(crate) fn reset_skill_configuration_scope(
    api: State<'_, SkillApi>,
    skill_id: String,
    input: SkillConfigurationScopeInput,
) -> Result<SkillConfigurationResolutionOutcome, CommandError> {
    let (key, scope) =
        mapper::configuration_scope_target(skill_id, input).map_err(map_command_error)?;
    api.reset_configuration_scope(&key, scope)
        .map(mapper::resolution_outcome_to_dto)
        .map_err(map_command_error)
}
