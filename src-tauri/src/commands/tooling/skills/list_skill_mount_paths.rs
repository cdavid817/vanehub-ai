use super::dto::SkillAgentMountPath;
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_skill_mount_paths(
    api: State<'_, SkillApi>,
) -> Result<Vec<SkillAgentMountPath>, CommandError> {
    api.list_mount_paths()
        .map(mapper::mount_paths_to_dto)
        .map_err(map_command_error)
}
