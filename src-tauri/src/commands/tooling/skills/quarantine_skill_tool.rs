use super::skill_tool_dto::{SkillToolQuarantineInput, SkillToolRevisionDto};
use super::skill_tool_mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skill_tools::api::SkillToolApi;
use tauri::State;

#[tauri::command]
pub(crate) fn quarantine_skill_tool(
    api: State<'_, SkillToolApi>,
    input: SkillToolQuarantineInput,
) -> Result<SkillToolRevisionDto, CommandError> {
    let revision = skill_tool_mapper::revision_id(&input.revision).map_err(map_command_error)?;
    api.quarantine(&revision, &input.reason)
        .map(skill_tool_mapper::revision)
        .map_err(map_command_error)
}
