use super::dto::{SkillConfigurationView, SkillScopeInput};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_skill_configuration(
    api: State<'_, SkillApi>,
    skill_id: String,
    input: SkillScopeInput,
) -> Result<SkillConfigurationView, CommandError> {
    let key = mapper::key(skill_id.clone(), input).map_err(map_command_error)?;
    api.configuration_view(key)
        .map(|view| mapper::configuration_view_to_dto(skill_id, view))
        .map_err(map_command_error)
}
