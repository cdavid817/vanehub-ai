use super::{dto, mapper};
use crate::contexts::execution_observability::api::ExecutionObservabilityApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_execution_observation_capabilities(
    api: State<'_, ExecutionObservabilityApi>,
) -> Vec<dto::ExecutionObservationCapabilityDto> {
    api.observation_capabilities()
        .into_iter()
        .map(mapper::capability_to_dto)
        .collect()
}
