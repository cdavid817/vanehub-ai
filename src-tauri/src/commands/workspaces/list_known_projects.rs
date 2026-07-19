use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_known_projects(
    api: State<'_, WorkspaceApi>,
) -> Result<Vec<dto::KnownProject>, CommandError> {
    api.list_known_projects()
        .map(|projects| {
            projects
                .into_iter()
                .map(mapper::known_project_to_dto)
                .collect()
        })
        .map_err(map_command_error)
}
