use super::{dto, mapper};
use crate::commands::error::CommandError;
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn save_onepiece_provider_profile(
    api: State<'_, AgentRuntimeApi>,
    input: dto::SaveOnePieceProviderProfileInput,
) -> Result<dto::OnePieceProviderProfiles, CommandError> {
    api.save_onepiece_provider_profile(mapper::save_onepiece_provider_profile_request(input))
        .map(mapper::onepiece_provider_profiles_to_dto)
        .map_err(CommandError::from)
}
