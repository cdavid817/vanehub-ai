use super::{evidence_dto, evidence_mapper};
use crate::contexts::execution_observability::api::evidence::{
    ExecutionEvidenceApi, ExecutionRecordDetailQuery,
};
use tauri::State;

/// Reads one execution record with its correlation counts.
///
/// The session is part of the query rather than derived from the record: a caller must already
/// know which session it is asking about, so a guessed record id cannot be used to enumerate
/// another session's work.
#[tauri::command]
pub(crate) fn get_execution_record(
    api: State<'_, ExecutionEvidenceApi>,
    session_id: String,
    record_id: String,
) -> Result<evidence_dto::ExecutionRecordDetailViewDto, evidence_dto::EvidenceCommandErrorDto> {
    execution_record_detail(api.inner(), session_id, record_id)
}

pub(super) fn execution_record_detail(
    api: &ExecutionEvidenceApi,
    session_id: String,
    record_id: String,
) -> Result<evidence_dto::ExecutionRecordDetailViewDto, evidence_dto::EvidenceCommandErrorDto> {
    let session_id = evidence_mapper::parse_session(&session_id)?;
    if record_id.trim().is_empty() {
        return Err(evidence_mapper::invalid_request());
    }
    let view = api
        .record_detail(ExecutionRecordDetailQuery {
            session_id: session_id.clone(),
            record_id,
        })
        .map_err(evidence_mapper::command_error)?;
    // The detail's coverage comes from the same store the record came from, so a caller cannot
    // receive a row while being told nothing about how complete the store's view of it is.
    let coverage = api
        .subscription_bootstrap(&session_id)
        .map_err(evidence_mapper::command_error)?
        .coverage;
    Ok(evidence_mapper::detail_view_dto(&view, &coverage))
}
