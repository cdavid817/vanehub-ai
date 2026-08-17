use super::{dto, mapper};
use crate::contexts::execution_observability::EvaluationApi;
use tauri::State;
#[tauri::command]
pub(crate) fn get_evaluation_attempt(
    api: State<'_, EvaluationApi>,
    attempt_id: String,
) -> Result<dto::EvaluationAttempt, String> {
    api.list(0, 100)
        .map_err(mapper::safe_error)?
        .into_iter()
        .flat_map(|arena| arena.attempts)
        .find(|attempt| attempt.id == attempt_id)
        .map(mapper::attempt)
        .ok_or_else(|| "evaluation attempt not found".into())
}
