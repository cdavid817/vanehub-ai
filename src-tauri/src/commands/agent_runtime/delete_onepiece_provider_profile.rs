use super::{dto, mapper};
use crate::commands::error::CommandError;
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn delete_onepiece_provider_profile(
    api: State<'_, AgentRuntimeApi>,
    profile_id: String,
) -> Result<dto::OnePieceProviderProfiles, CommandError> {
    api.delete_onepiece_provider_profile(&profile_id)
        .map(mapper::onepiece_provider_profiles_to_dto)
        .map_err(CommandError::from)
}
