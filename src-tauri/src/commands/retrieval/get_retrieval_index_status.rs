//! `add-retrieval-vector-search` Task 15: reports index status for one agent, aggregated across
//! *all* of that agent's `scope_folder` rows (design doc §7.4) — unlike configuration, this is
//! scoped per agent rather than global.

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
    agent_id: String,
) -> Result<RetrievalIndexStatus, String> {
    api.index_status(&agent_id)
        .map(|status| RetrievalIndexStatus {
            indexed: status.indexed,
            pending: status.pending,
            failed: status.failed,
            last_failure_category: status.last_failure_category,
        })
        .map_err(|error| error.to_string())
}
