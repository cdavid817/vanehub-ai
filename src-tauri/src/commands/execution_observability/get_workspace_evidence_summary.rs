use super::{evidence_dto, evidence_mapper};
use crate::contexts::execution_observability::api::evidence::{
    ExecutionEvidenceApi, WorkspaceEvidenceSummaryQuery,
};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

/// Reads the evidence summary for one session.
///
/// The handler parses, calls the published API, and maps. It issues no SQL, decodes no cursor,
/// touches no repository, and never sees a raw payload — everything that could widen a query or
/// leak content stays behind `execution_observability::api`.
///
/// The live Shell figure comes from the workspaces API rather than from the evidence journal, and
/// it is joined here because this is the one place that holds both. Evidence records what has
/// already happened; a Shell that is open right now is a fact only its registry has, and asking
/// the journal for it would answer with the last thing that was written about a Shell instead of
/// whether one is still there.
#[tauri::command]
pub(crate) fn get_workspace_evidence_summary(
    api: State<'_, ExecutionEvidenceApi>,
    workspaces: State<'_, WorkspaceApi>,
    session_id: String,
    seat_id: Option<String>,
) -> Result<evidence_dto::WorkspaceEvidenceSummaryDto, evidence_dto::EvidenceCommandErrorDto> {
    let live_shells = workspaces.live_session_shell_count(&session_id);
    workspace_evidence_summary(api.inner(), session_id, seat_id, live_shells)
}

/// The body, separated from the `State` wrapper so tests exercise this code rather than a copy of
/// it. `State` cannot be constructed outside a running app, and a test that re-implemented the
/// handler would pass while the registered one drifted.
pub(super) fn workspace_evidence_summary(
    api: &ExecutionEvidenceApi,
    session_id: String,
    seat_id: Option<String>,
    live_shells: usize,
) -> Result<evidence_dto::WorkspaceEvidenceSummaryDto, evidence_dto::EvidenceCommandErrorDto> {
    let scope = evidence_mapper::parse_scope(evidence_dto::EvidenceScopeDto {
        session_id: Some(session_id),
        seat_id,
        ..Default::default()
    })?;
    let query = WorkspaceEvidenceSummaryQuery {
        session_id: scope
            .session_id
            .ok_or_else(evidence_mapper::invalid_request)?,
        seat_id: scope.seat_id,
    };
    api.summary(query)
        .map(|summary| evidence_mapper::summary_dto(&summary, live_shells))
        .map_err(evidence_mapper::command_error)
}
