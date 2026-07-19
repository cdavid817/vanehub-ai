use super::{dto, mapper};
use crate::commands::error::CommandError;
use crate::contexts::tooling::plugin_integrations::api::PluginIntegrationApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_plugin_integration_overview(
    api: State<'_, PluginIntegrationApi>,
) -> Result<dto::PluginIntegrationOverview, CommandError> {
    Ok(mapper::overview_to_dto(api.overview()))
}
