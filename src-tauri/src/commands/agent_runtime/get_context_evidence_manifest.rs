use super::{context_manifest_mapper, dto};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_context_evidence_manifest(
    api: State<'_, AgentRuntimeApi>,
    generation_id: String,
) -> Result<Option<dto::ContextEvidenceManifest>, CommandError> {
    api.get_context_evidence_manifest(&generation_id)
        .map(|manifest| manifest.map(context_manifest_mapper::manifest_to_dto))
        .map_err(map_command_error)
}
