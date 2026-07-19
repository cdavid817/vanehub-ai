use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_known_remote_workspaces(
    api: State<'_, WorkspaceApi>,
) -> Result<Vec<dto::KnownRemoteWorkspace>, CommandError> {
    api.list_known_remote_workspaces()
        .map(|workspaces| {
            workspaces
                .into_iter()
                .map(mapper::known_remote_workspace_to_dto)
                .collect()
        })
        .map_err(map_command_error)
}
