use super::{evidence_dto, evidence_mapper};
use crate::contexts::execution_observability::api::evidence::ExecutionEvidenceApi;
use tauri::State;

/// Returns the sequence the store has already committed through.
///
/// A subscriber registers its listener first and calls this second. The watermark is what lets it
/// discard the notices that arrived during that window without also discarding the ones that
/// describe work it has not seen — the two are indistinguishable without it.
#[tauri::command]
pub(crate) fn get_evidence_subscription_bootstrap(
    api: State<'_, ExecutionEvidenceApi>,
    session_id: String,
) -> Result<evidence_dto::EvidenceSubscriptionBootstrapDto, evidence_dto::EvidenceCommandErrorDto> {
    evidence_subscription_bootstrap(api.inner(), session_id)
}

pub(super) fn evidence_subscription_bootstrap(
    api: &ExecutionEvidenceApi,
    session_id: String,
) -> Result<evidence_dto::EvidenceSubscriptionBootstrapDto, evidence_dto::EvidenceCommandErrorDto> {
    let session_id = evidence_mapper::parse_session(&session_id)?;
    api.subscription_bootstrap(&session_id)
        .map(|bootstrap| evidence_mapper::bootstrap_dto(&bootstrap))
        .map_err(evidence_mapper::command_error)
}
