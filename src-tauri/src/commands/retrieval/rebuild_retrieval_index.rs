//! `add-retrieval-vector-search` Task 15: rebuilds the retrieval index for one agent, aggregated
//! across *all* of that agent's `scope_folder` rows (design doc §7.4) — unlike configuration,
//! this is scoped per agent rather than global.
//!
//! Requeues rows and wakes the background worker; it does not synchronously run embeddings on the
//! command thread (`RetrievalApi::rebuild`'s own doc comment), so this returns as soon as the
//! requeue is durable.

use crate::contexts::retrieval::api::RetrievalApi;
use tauri::State;

#[tauri::command]
pub(crate) fn rebuild_retrieval_index(
    api: State<'_, RetrievalApi>,
    agent_id: String,
) -> Result<(), String> {
    api.rebuild(&agent_id).map_err(|error| error.to_string())
}
