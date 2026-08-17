use super::skill_tool_dto::{SkillToolEnablementInput, SkillToolRevisionDto};
use super::skill_tool_mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skill_tools::api::SkillToolApi;
use tauri::State;

#[tauri::command]
pub(crate) fn set_skill_tool_enabled(
    api: State<'_, SkillToolApi>,
    input: SkillToolEnablementInput,
) -> Result<SkillToolRevisionDto, CommandError> {
    let revision = skill_tool_mapper::revision_id(&input.revision).map_err(map_command_error)?;
    api.set_enabled(&revision, input.enabled)
        .map(skill_tool_mapper::revision)
        .map_err(map_command_error)
}
