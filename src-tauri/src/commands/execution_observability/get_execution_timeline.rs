use super::{dto, mapper};
use crate::contexts::execution_observability::api::{ExecutionObservabilityApi, ExecutionRunId};
use tauri::State;

#[tauri::command]
/// One run's timeline, with its event list bounded and resumable.
///
/// `event_page_token` resumes the event list after a cursor a previous response reported. It is
/// absent for the ordinary first read, so existing callers are unaffected: the run and its spans
/// are returned identically either way and only the event page moves.
pub(crate) fn get_execution_timeline(
    api: State<'_, ExecutionObservabilityApi>,
    run_id: String,
    event_page_token: Option<String>,
) -> Result<dto::ExecutionTimelineDto, dto::ObservabilityCommandErrorDto> {
    let run_id = ExecutionRunId::parse(run_id).map_err(|_| mapper::run_not_found())?;
    api.timeline_from_event(&run_id, event_page_token.as_deref())
        .map_err(mapper::adapter_error)?
        .map(mapper::timeline_to_dto)
        .ok_or_else(mapper::run_not_found)
}
