//! `add-retrieval-vector-search` Task 15: exposes the global retrieval configuration singleton
//! (design doc §7.4) to the frontend. This DTO is deliberately distinct from
//! `retrieval::application::ports::RetrievalConfiguration` — the command layer only ever sees
//! `retrieval::api`, never that context's application/domain internals (`api.rs`'s own module
//! doc comment).

use crate::contexts::retrieval::api::RetrievalApi;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RetrievalConfiguration {
    pub(crate) source_profile_id: Option<String>,
    pub(crate) embedding_model: Option<String>,
}

#[tauri::command]
pub(crate) fn get_retrieval_configuration(
    api: State<'_, RetrievalApi>,
) -> Result<RetrievalConfiguration, String> {
    api.configuration()
        .map(|configuration| RetrievalConfiguration {
            source_profile_id: configuration.source_profile_id,
            embedding_model: configuration.embedding_model,
        })
        // 类别而非 `Display`：`RetrievalError::Storage` 的载荷是 rusqlite 的原始消息，而
        // 这个串会被 `onepiece-retrieval-section.tsx` 原样渲染进 DOM（设计文档 §8.2）。
        .map_err(|error| error.category().to_string())
}
