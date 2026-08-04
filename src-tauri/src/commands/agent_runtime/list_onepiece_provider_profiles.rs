use super::{dto, mapper};
use crate::commands::error::CommandError;
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_onepiece_provider_profiles(
    api: State<'_, AgentRuntimeApi>,
) -> Result<dto::OnePieceProviderProfiles, CommandError> {
    api.onepiece_provider_profiles()
        .map(mapper::onepiece_provider_profiles_to_dto)
        .map_err(CommandError::from)
}
