use super::dto::safe_error;
use crate::contexts::retrieval::api::CodeIndexApi;
use tauri::State;

#[tauri::command]
pub(crate) async fn delete_code_index_workspace(
    workspace_id: String,
    api: State<'_, CodeIndexApi>,
) -> Result<(), String> {
    api.delete(workspace_id).await.map_err(safe_error)
}
