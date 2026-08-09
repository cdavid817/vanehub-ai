use super::dto::{safe_error, CodeIndexAuditDto};
use crate::contexts::retrieval::api::CodeIndexApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_code_index_audit(
    workspace_id: String,
    limit: Option<usize>,
    api: State<'_, CodeIndexApi>,
) -> Result<Vec<CodeIndexAuditDto>, String> {
    api.audit(&workspace_id, limit.unwrap_or(50))
        .map(|entries| entries.into_iter().map(CodeIndexAuditDto::from).collect())
        .map_err(safe_error)
}
