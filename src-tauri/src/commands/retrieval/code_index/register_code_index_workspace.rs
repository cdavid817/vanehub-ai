use super::dto::{safe_error, CodeIndexWorkspaceDto};
use crate::contexts::retrieval::api::CodeIndexApi;
use std::path::Path;
use tauri::State;

#[tauri::command]
pub(crate) fn register_code_index_workspace(
    root: String,
    display_name: String,
    api: State<'_, CodeIndexApi>,
) -> Result<CodeIndexWorkspaceDto, String> {
    let workspace = api
        .register_workspace(Path::new(&root), &display_name)
        .map_err(safe_error)?;
    let status = api
        .workspace_status(&workspace.workspace_id)
        .map_err(safe_error)?;
    Ok(CodeIndexWorkspaceDto::new(workspace, status))
}
