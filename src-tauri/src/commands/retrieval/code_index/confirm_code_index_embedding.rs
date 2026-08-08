use super::dto::{safe_error, CodeEmbeddingConfirmationDto};
use crate::contexts::retrieval::api::CodeIndexApi;
use tauri::State;

#[tauri::command]
pub(crate) fn confirm_code_index_embedding(
    workspace_id: String,
    profile_id: String,
    model: String,
    generation: u64,
    api: State<'_, CodeIndexApi>,
) -> Result<CodeEmbeddingConfirmationDto, String> {
    api.confirm_embedding(&workspace_id, &profile_id, &model, generation)
        .map(CodeEmbeddingConfirmationDto::from)
        .map_err(safe_error)
}
