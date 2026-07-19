use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn select_workspace_directory(
    api: State<'_, SkillApi>,
) -> Result<Option<String>, CommandError> {
    api.select_workspace_directory().map_err(map_command_error)
}
