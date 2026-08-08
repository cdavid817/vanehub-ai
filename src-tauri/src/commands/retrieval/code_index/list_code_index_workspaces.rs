use super::dto::{safe_error, CodeIndexWorkspaceDto};
use crate::contexts::retrieval::api::CodeIndexApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_code_index_workspaces(
    api: State<'_, CodeIndexApi>,
) -> Result<Vec<CodeIndexWorkspaceDto>, String> {
    api.list_workspaces()
        .and_then(|workspaces| {
            workspaces
                .into_iter()
                .map(|workspace| {
                    let status = api.workspace_status(&workspace.workspace_id)?;
                    Ok(CodeIndexWorkspaceDto::new(workspace, status))
                })
                .collect()
        })
        .map_err(safe_error)
}
