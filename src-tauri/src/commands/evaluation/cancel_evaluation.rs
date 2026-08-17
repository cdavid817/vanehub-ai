use super::{dto, mapper};
use crate::contexts::execution_observability::EvaluationApi;
use tauri::State;
#[tauri::command]
pub(crate) fn cancel_evaluation(
    api: State<'_, EvaluationApi>,
    arena_id: String,
) -> Result<dto::EvaluationArena, String> {
    api.cancel(&arena_id)
        .map(mapper::arena)
        .map_err(mapper::safe_error)
}
