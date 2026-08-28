use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::personalization::api::PersonalizationApi;
use tauri::State;

/// Every registered Agent and the surface it can consume.
///
/// A screen renders its controls from this rather than from a list of Agent ids it carries: an
/// Agent the user registered themselves gets the same treatment as one that shipped.
#[tauri::command]
pub(crate) fn list_personalization_agent_capabilities(
    api: State<'_, PersonalizationApi>,
) -> Result<Vec<dto::AgentCapabilityView>, CommandError> {
    api.agent_capabilities()
        .map(mapper::capabilities_to_dto)
        .map_err(map_command_error)
}
