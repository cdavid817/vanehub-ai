use super::skill_tool_dto::{SkillToolRevisionDto, SkillToolTrustInput};
use super::skill_tool_mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skill_tools::api::SkillToolApi;
use tauri::State;

#[tauri::command]
pub(crate) fn set_skill_tool_trust(
    api: State<'_, SkillToolApi>,
    input: SkillToolTrustInput,
) -> Result<SkillToolRevisionDto, CommandError> {
    let revision = skill_tool_mapper::revision_id(&input.revision).map_err(map_command_error)?;
    let state = api.diagnostics(&revision).map_err(map_command_error)?;
    api.decide_trust(&state.key, &state.integrity, &input.actor, input.trusted)
        .map(skill_tool_mapper::revision)
        .map_err(map_command_error)
}
