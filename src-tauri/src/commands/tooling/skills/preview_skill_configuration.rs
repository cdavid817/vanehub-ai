use super::dto::{SkillConfigurationResolutionOutcome, SkillConfigurationWriteInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

/// Validates a draft and resolves what saving it would produce, without writing anything. A
/// rejection is returned as an outcome rather than a command error because the caller has to keep
/// its draft and act on the reason.
#[tauri::command]
pub(crate) fn preview_skill_configuration(
    api: State<'_, SkillApi>,
    skill_id: String,
    input: SkillConfigurationWriteInput,
) -> Result<SkillConfigurationResolutionOutcome, CommandError> {
    let (key, request) =
        mapper::configuration_request(skill_id, input).map_err(map_command_error)?;
    api.preview_configuration(&key, &request)
        .map(mapper::resolution_outcome_to_dto)
        .map_err(map_command_error)
}
