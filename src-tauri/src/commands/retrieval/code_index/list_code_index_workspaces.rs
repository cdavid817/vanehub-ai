use super::dto::safe_error;
use super::dto::CodeIndexWorkspaceDto;
use crate::contexts::retrieval::api::CodeIndexApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_code_index_workspaces(
    api: State<'_, CodeIndexApi>,
) -> Result<Vec<CodeIndexWorkspaceDto>, String> {
    // One query for every workspace's status instead of one workspace_status() call per
    // workspace (each of which runs ~10 COUNT subqueries).
    api.list_workspaces_with_status()
        .map(|workspaces| {
            workspaces
                .into_iter()
                .map(|(workspace, status)| CodeIndexWorkspaceDto::new(workspace, status))
                .collect()
        })
        .map_err(safe_error)
}
