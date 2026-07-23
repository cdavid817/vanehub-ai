use super::{dto, mapper};
use crate::contexts::execution_observability::api::{ExecutionObservabilityApi, ExecutionRunId};
use tauri::State;

#[tauri::command]
pub(crate) fn get_execution_timeline(
    api: State<'_, ExecutionObservabilityApi>,
    run_id: String,
) -> Result<dto::ExecutionTimelineDto, dto::ObservabilityCommandErrorDto> {
    let run_id = ExecutionRunId::parse(run_id).map_err(|_| mapper::run_not_found())?;
    api.timeline(&run_id)
        .map_err(mapper::adapter_error)?
        .map(mapper::timeline_to_dto)
        .ok_or_else(mapper::run_not_found)
}
