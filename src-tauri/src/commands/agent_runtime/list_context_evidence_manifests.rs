use super::{context_manifest_mapper, dto};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_context_evidence_manifests(
    api: State<'_, AgentRuntimeApi>,
    input: dto::ContextManifestQuery,
) -> Result<dto::ContextEvidenceManifestPage, CommandError> {
    api.list_context_evidence_manifests(
        input.session_id.as_deref(),
        input.cursor.as_deref(),
        input.limit,
    )
    .map(context_manifest_mapper::page_to_dto)
    .map_err(map_command_error)
}
