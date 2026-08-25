use tauri::State;

use super::background;
use super::dto::CliOperationHandleDto;
use super::error::{command_error, CliEnvironmentCommandError};
use crate::contexts::tooling::cli::api::CliEnvironmentApi;

/// Re-detects one or more CLI environments.
///
/// Returns before path enumeration, version probes, or catalog queries complete. An empty
/// `agentIds` means every registered tool.
#[tauri::command]
pub(crate) fn refresh_cli_environment(
    api: State<'_, CliEnvironmentApi>,
    agent_ids: Vec<String>,
    force_catalog: bool,
) -> Result<CliOperationHandleDto, CliEnvironmentCommandError> {
    // Validated before the operation exists, so a typo does not leave a failed operation behind.
    let prepared = api
        .prepare_refresh(agent_ids, force_catalog)
        .map_err(command_error)?;
    let operation_id = prepared.operation_id.clone();
    background::spawn_refresh(api.inner().clone(), prepared);
    Ok(CliOperationHandleDto { operation_id })
}
