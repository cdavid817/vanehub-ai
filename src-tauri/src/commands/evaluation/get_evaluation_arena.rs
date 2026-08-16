use super::{dto, mapper};
use crate::contexts::execution_observability::EvaluationApi;
use tauri::State;
#[tauri::command]
pub(crate) fn get_evaluation_arena(
    api: State<'_, EvaluationApi>,
    arena_id: String,
) -> Result<dto::EvaluationArena, String> {
    api.get(&arena_id)
        .map(mapper::arena)
        .map_err(mapper::safe_error)
}
