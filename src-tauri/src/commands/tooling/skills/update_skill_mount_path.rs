use super::dto::SkillMountMigrationReport;
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn update_skill_mount_path(
    api: State<'_, SkillApi>,
    agent_id: String,
    mount_path: String,
) -> Result<SkillMountMigrationReport, CommandError> {
    let mount_path = mapper::mount_path(mount_path).map_err(map_command_error)?;
    api.update_mount_path(agent_id, mount_path)
        .map(mapper::mount_migration_to_dto)
        .map_err(map_command_error)
}
