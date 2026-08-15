use super::dto::{SkillConfigurationSaveOutcome, SkillConfigurationWriteInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn clear_skill_configuration_secret(
    api: State<'_, SkillApi>,
    skill_id: String,
    input: SkillConfigurationWriteInput,
    property_key: String,
) -> Result<SkillConfigurationSaveOutcome, CommandError> {
    let (key, request) =
        mapper::configuration_request(skill_id, input).map_err(map_command_error)?;
    api.clear_configuration_secret(&key, &request, &property_key)
        .map(mapper::save_outcome_to_dto)
        .map_err(map_command_error)
}
