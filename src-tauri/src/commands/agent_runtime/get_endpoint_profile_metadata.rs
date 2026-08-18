use super::{dto, mapper};
use crate::commands::error::CommandError;
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_endpoint_profile_metadata(
    api: State<'_, AgentRuntimeApi>,
    profile_id: String,
) -> Result<Option<dto::EndpointProfileMetadata>, CommandError> {
    api.endpoint_profile_metadata(&profile_id)
        .map(|value| value.map(mapper::endpoint_profile_metadata_to_dto))
        .map_err(CommandError::from)
}
