use super::dto::{safe_error, CodeIndexStatusDto};
use crate::contexts::retrieval::api::CodeIndexApi;
use tauri::State;

#[tauri::command]
pub(crate) fn refresh_code_index_workspace(
    workspace_id: String,
    api: State<'_, CodeIndexApi>,
) -> Result<CodeIndexStatusDto, String> {
    api.refresh(&workspace_id)
        .map(CodeIndexStatusDto::from)
        .map_err(safe_error)
}
