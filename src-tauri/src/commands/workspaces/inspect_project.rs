use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) fn inspect_project(
    api: State<'_, WorkspaceApi>,
    path: String,
) -> Result<dto::ProjectInspection, CommandError> {
    api.inspect_project(&path)
        .map(mapper::project_inspection_to_dto)
        .map_err(map_command_error)
}
