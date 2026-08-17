use super::{dto, mapper};
use crate::contexts::execution_observability::{EvaluationApi, StartEvaluationRequest};
use tauri::State;

#[tauri::command]
pub(crate) fn start_evaluation(
    api: State<'_, EvaluationApi>,
    input: dto::StartEvaluationInput,
) -> Result<dto::EvaluationArena, String> {
    api.start_async(StartEvaluationRequest {
        task_id: input.task_id,
        task_version: input.task_version,
        agent_ids: input.agent_ids,
    })
    .map(mapper::arena)
    .map_err(mapper::safe_error)
}
