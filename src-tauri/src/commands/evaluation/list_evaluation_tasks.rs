use super::{dto, mapper};
use crate::contexts::execution_observability::EvaluationApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_evaluation_tasks(
    api: State<'_, EvaluationApi>,
) -> Result<Vec<dto::EvaluationTask>, String> {
    api.list_tasks()
        .map_err(mapper::safe_error)
        .map(|items| items.into_iter().map(mapper::task).collect())
}
