//! `add-retrieval-vector-search` Task 15: lists embedding-capable models for a saved OnePiece
//! provider Profile, optionally against a transient (typed-but-not-yet-saved) credential so the
//! settings page can list models before the user saves their API key.
//!
//! This command reaches into `agent_runtime::api` rather than `retrieval::api`: listing models is
//! entirely a Profile/credential/provider-catalog concern, which `agent_runtime` owns end to end
//! (design doc §4.3) — `retrieval` never learns about Profiles or credentials at all. `retrieval`
//! and `agent_runtime` never import each other, but this command file is neither of those
//! contexts, so it is free to depend on `agent_runtime::api` directly, the same way
//! `commands::agent_runtime::delete_agent_memory` composes both `agent_runtime::api` and
//! `retrieval::api` (that file's own module doc comment).
//!
//! The credential itself never crosses this boundary in either direction beyond this call: it is
//! taken as an optional input to be forwarded to the provider, and the return type below carries
//! only `id`/`display_name` — never a credential, and never the discovery `source` tag that
//! `agent_runtime::api::OnePieceProviderModelOption` also carries internally.

use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmbeddingModelOption {
    pub(crate) id: String,
    pub(crate) display_name: String,
}

#[tauri::command]
pub(crate) async fn list_embedding_models(
    api: State<'_, AgentRuntimeApi>,
    profile_id: String,
    transient_credential: Option<String>,
) -> Result<Vec<EmbeddingModelOption>, String> {
    api.list_embedding_models(&profile_id, transient_credential.as_deref())
        .await
        .map(|models| {
            models
                .into_iter()
                .map(|model| EmbeddingModelOption {
                    id: model.id,
                    display_name: model.display_name,
                })
                .collect()
        })
        .map_err(|error| error.to_string())
}
