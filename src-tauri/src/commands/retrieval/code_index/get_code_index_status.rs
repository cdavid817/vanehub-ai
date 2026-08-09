use super::dto::{safe_error, CodeIndexStatusDto};
use crate::contexts::retrieval::api::CodeIndexApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_code_index_status(
    workspace_id: String,
    api: State<'_, CodeIndexApi>,
) -> Result<CodeIndexStatusDto, String> {
    api.workspace_status(&workspace_id)
        .map(CodeIndexStatusDto::from)
        .map_err(safe_error)
}
