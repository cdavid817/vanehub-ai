use super::{dto, mapper};
use crate::contexts::execution_observability::EvaluationApi;
use tauri::State;
#[tauri::command]
pub(crate) fn list_evaluation_arenas(
    api: State<'_, EvaluationApi>,
) -> Result<Vec<dto::EvaluationArena>, String> {
    api.list(0, 100)
        .map_err(mapper::safe_error)
        .map(|items| items.into_iter().map(mapper::arena).collect())
}
