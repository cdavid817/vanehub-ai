use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::plugin_integrations::api::PluginIntegrationApi;
use tauri::State;

#[tauri::command]
pub(crate) fn test_plugin_integration(
    api: State<'_, PluginIntegrationApi>,
    request: dto::PluginIntegrationRequest,
) -> Result<dto::PluginIntegrationTestResult, CommandError> {
    api.test_readiness(mapper::request_id(request))
        .map(mapper::test_result_to_dto)
        .map_err(map_command_error)
}
