use super::{dto, mapper};
use crate::commands::error::CommandError;
use crate::contexts::tooling::plugin_integrations::api::PluginIntegrationApi;
use tauri::State;

#[tauri::command]
pub(crate) fn refresh_plugin_integrations(
    api: State<'_, PluginIntegrationApi>,
) -> Result<dto::PluginIntegrationOverview, CommandError> {
    Ok(mapper::overview_to_dto(api.refresh()))
}
