//! `add-retrieval-vector-search` Task 15: rebuilds the whole retrieval index, across every agent
//! and every `scope_folder`. Global like the configuration singleton and the status it pairs with
//! — a per-agent filter would leave other agents' rows unreachable by the only rebuild the UI
//! exposes.
//!
//! Requeues rows and wakes the background worker; it does not synchronously run embeddings on the
//! command thread (`RetrievalApi::rebuild`'s own doc comment), so this returns as soon as the
//! requeue is durable.

use crate::contexts::retrieval::api::RetrievalApi;
use tauri::State;

#[tauri::command]
pub(crate) fn rebuild_retrieval_index(api: State<'_, RetrievalApi>) -> Result<(), String> {
    // 类别而非 `Display`：这个串会被设置页原样渲染（设计文档 §8.2）。
    api.rebuild().map_err(|error| error.category().to_string())
}
