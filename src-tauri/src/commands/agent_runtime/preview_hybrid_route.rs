use super::{dto, mapper};
use crate::commands::error::CommandError;
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn preview_hybrid_route(
    api: State<'_, AgentRuntimeApi>,
    input: dto::HybridRoutePreviewInput,
) -> Result<dto::HybridRoutePreview, CommandError> {
    api.preview_hybrid_route(mapper::hybrid_preview_request(input))
        .map(mapper::hybrid_preview_to_dto)
        .map_err(CommandError::from)
}
