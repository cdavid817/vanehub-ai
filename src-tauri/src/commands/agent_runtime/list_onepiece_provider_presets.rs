use crate::commands::agent_runtime::{dto, mapper};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_onepiece_provider_presets(
    api: State<'_, AgentRuntimeApi>,
) -> Vec<dto::OnePieceProviderPreset> {
    mapper::onepiece_provider_presets_to_dto(api.onepiece_provider_presets())
}
