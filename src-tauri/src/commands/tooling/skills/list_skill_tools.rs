use super::skill_tool_dto::{SkillToolOwnerInput, SkillToolRevisionDto};
use super::skill_tool_mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skill_tools::api::SkillToolApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_skill_tools(
    api: State<'_, SkillToolApi>,
    input: SkillToolOwnerInput,
) -> Result<Vec<SkillToolRevisionDto>, CommandError> {
    let owner = skill_tool_mapper::owner(
        &input.skill_id,
        &input.scope,
        input.workspace_path.as_deref(),
    )
    .map_err(map_command_error)?;
    api.list(&owner)
        .map(|items| items.into_iter().map(skill_tool_mapper::revision).collect())
        .map_err(map_command_error)
}
