use super::dto::{SkillConfigurationSaveOutcome, SkillConfigurationWriteInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

/// Removes one non-secret property from one scope so it inherits again. Clearing a secret is a
/// separate operation because it destroys a credential rather than a stored value.
#[tauri::command]
pub(crate) fn reset_skill_configuration_property(
    api: State<'_, SkillApi>,
    skill_id: String,
    input: SkillConfigurationWriteInput,
    property_key: String,
) -> Result<SkillConfigurationSaveOutcome, CommandError> {
    let (key, request) =
        mapper::configuration_request(skill_id, input).map_err(map_command_error)?;
    api.reset_configuration_property(&key, &request, &property_key)
        .map(mapper::save_outcome_to_dto)
        .map_err(map_command_error)
}
