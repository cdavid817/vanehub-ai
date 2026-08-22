use tauri::State;

use super::background;
use super::dto::CliOperationHandleDto;
use super::error::{command_error, CliEnvironmentCommandError};
use crate::contexts::tooling::cli::api::CliEnvironmentApi;
use crate::contexts::tooling::cli::application::environment_planning::PrepareCliActionInput;
use crate::contexts::tooling::cli::domain::action::CliActionKind;

/// Prepares a single-use action plan from the source, channel, and target the user chose.
///
/// The plan is what carries the selected version forward. Nothing downstream reconstructs a command
/// from these arguments, which is why execution takes only a plan id.
#[tauri::command]
pub(crate) fn prepare_cli_action(
    api: State<'_, CliEnvironmentApi>,
    agent_id: String,
    action: String,
    source_id: String,
    target_version: Option<String>,
    channel: Option<String>,
) -> Result<CliOperationHandleDto, CliEnvironmentCommandError> {
    let action = parse_action(&action)?;
    let prepared = api
        .prepare_action(PrepareCliActionInput {
            agent_id,
            action,
            source_id,
            target_version,
            channel,
        })
        .map_err(command_error)?;
    let operation_id = prepared.operation_id.clone();
    background::spawn_action_planning(api.inner().clone(), prepared);
    Ok(CliOperationHandleDto { operation_id })
}

/// Resolves the wire action name, refusing anything the backend does not model.
///
/// A free-form string reaching the planner would be a way to ask for an action no source declared.
fn parse_action(value: &str) -> Result<CliActionKind, CliEnvironmentCommandError> {
    CliActionKind::ALL
        .into_iter()
        .find(|candidate| candidate.as_str() == value)
        .ok_or_else(|| {
            command_error(
                crate::contexts::tooling::cli::api::CliEnvironmentError::Validation(format!(
                    "unknown CLI action `{value}`"
                )),
            )
        })
}
