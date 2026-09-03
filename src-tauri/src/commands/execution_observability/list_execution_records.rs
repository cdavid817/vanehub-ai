use super::{evidence_dto, evidence_mapper};
use crate::contexts::execution_observability::api::evidence::{
    ExecutionEvidenceApi, ExecutionRecordQuery,
};
use tauri::State;

/// Reads one bounded page of execution records.
///
/// The cursor is passed through opaquely. This layer cannot decode it and does not try: a handler
/// that could read a cursor could also synthesise one, which is how offset arithmetic re-enters a
/// keyset pager and starts skipping rows across a concurrent append.
#[tauri::command]
pub(crate) fn list_execution_records(
    api: State<'_, ExecutionEvidenceApi>,
    scope: evidence_dto::EvidenceScopeDto,
    filters: Option<evidence_dto::EvidenceFiltersDto>,
    cursor: Option<String>,
    limit: Option<u32>,
) -> Result<evidence_dto::ExecutionRecordPageDto, evidence_dto::EvidenceCommandErrorDto> {
    execution_record_page(api.inner(), scope, filters, cursor, limit)
}

pub(super) fn execution_record_page(
    api: &ExecutionEvidenceApi,
    scope: evidence_dto::EvidenceScopeDto,
    filters: Option<evidence_dto::EvidenceFiltersDto>,
    cursor: Option<String>,
    limit: Option<u32>,
) -> Result<evidence_dto::ExecutionRecordPageDto, evidence_dto::EvidenceCommandErrorDto> {
    let query = ExecutionRecordQuery {
        scope: evidence_mapper::parse_scope(scope)?,
        filters: evidence_mapper::parse_filters(filters.unwrap_or_default())?,
        cursor: cursor.filter(|value| !value.is_empty()),
        limit: evidence_mapper::clamp_limit(limit),
    };
    api.list_records(query)
        .map(|page| evidence_mapper::page_dto(&page))
        .map_err(evidence_mapper::command_error)
}
