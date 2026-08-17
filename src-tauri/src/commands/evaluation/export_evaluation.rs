use super::{dto, mapper};
use crate::contexts::execution_observability::EvaluationApi;
use tauri::State;
#[tauri::command]
pub(crate) fn export_evaluation(
    api: State<'_, EvaluationApi>,
    arena_id: String,
) -> Result<dto::EvaluationExport, String> {
    api.export(&arena_id)
        .map(mapper::export)
        .map_err(mapper::safe_error)
}
