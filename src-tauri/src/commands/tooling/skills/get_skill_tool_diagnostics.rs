use super::skill_tool_dto::{SkillToolRevisionDto, SkillToolRevisionInput};
use super::skill_tool_mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skill_tools::api::SkillToolApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_skill_tool_diagnostics(
    api: State<'_, SkillToolApi>,
    input: SkillToolRevisionInput,
) -> Result<SkillToolRevisionDto, CommandError> {
    let revision = skill_tool_mapper::revision_id(&input.revision).map_err(map_command_error)?;
    api.diagnostics(&revision)
        .map(skill_tool_mapper::revision)
        .map_err(map_command_error)
}
