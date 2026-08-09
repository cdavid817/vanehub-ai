use super::dto::{safe_error, CodeIndexWorkspaceDto};
use crate::contexts::retrieval::api::CodeIndexApi;
use tauri::State;

#[tauri::command]
pub(crate) fn disable_code_index_workspace(
    workspace_id: String,
    api: State<'_, CodeIndexApi>,
) -> Result<CodeIndexWorkspaceDto, String> {
    let workspace = api.disable(&workspace_id).map_err(safe_error)?;
    let status = api.workspace_status(&workspace_id).map_err(safe_error)?;
    Ok(CodeIndexWorkspaceDto::new(workspace, status))
}
