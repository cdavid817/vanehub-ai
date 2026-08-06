//! `add-retrieval-vector-search` Task 15: reports index status, aggregated across every agent and
//! every `scope_folder`. Global like the configuration singleton it belongs to: retrieval applies
//! to all agents, so a per-agent filter would hide other agents' rows from the only surface that
//! reports them.

use crate::contexts::retrieval::api::RetrievalApi;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RetrievalIndexStatus {
    pub(crate) indexed: u32,
    pub(crate) pending: u32,
    pub(crate) failed: u32,
    /// Category only (e.g. `auth` / `invalid_request` / `rate_limit` / `network`) — never raw
    /// error text, which may carry credentials or provider response content (design doc §8.2).
    pub(crate) last_failure_category: Option<String>,
}

#[tauri::command]
pub(crate) fn get_retrieval_index_status(
    api: State<'_, RetrievalApi>,
) -> Result<RetrievalIndexStatus, String> {
    api.index_status()
        .map(|status| RetrievalIndexStatus {
            indexed: status.indexed,
            pending: status.pending,
            failed: status.failed,
            last_failure_category: status.last_failure_category,
        })
        .map_err(|error| error.to_string())
}
