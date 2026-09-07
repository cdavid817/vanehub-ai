use tauri::State;

use super::dto::CliConnectionCheckDto;
use super::error::{command_error, CliEnvironmentCommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::agent_runtime::application::ManagedConnectionCheckRequest;
use crate::contexts::tooling::cli::api::CliApi;
use crate::platform::logging::redact_text;

/// An explicit, user-initiated ACP handshake with the installed program.
///
/// Separate from detection on purpose: detection runs only the read-only version probe, and a
/// handshake starts the vendor's agent. This is the one place that start is allowed to happen
/// from the management page, and it does `initialize` only -- no session, no prompt -- before
/// the process is released. The executable is the one detection resolved; a bare command name is
/// never re-resolved inside the child.
#[tauri::command]
pub(crate) fn check_cli_connection(
    api: State<'_, CliApi>,
    agent_runtime: State<'_, AgentRuntimeApi>,
    agent_id: String,
    provider_id: Option<String>,
) -> Result<CliConnectionCheckDto, CliEnvironmentCommandError> {
    let executable = api
        .resolve_executable(&agent_id)
        .map_err(command_error)?
        .ok_or_else(|| check_failure("connection-check-not-installed", &agent_id))?;
    let workspace = std::env::temp_dir().to_string_lossy().to_string();
    let report = agent_runtime
        .check_managed_connection(ManagedConnectionCheckRequest {
            agent_id: agent_id.clone(),
            executable,
            workspace,
            provider_id,
        })
        .map_err(|message| check_failure("connection-check-failed", &message))?;
    Ok(CliConnectionCheckDto {
        agent_id: report.agent_id,
        transport: report.transport,
        protocol_version: report.protocol_version,
        load_session: report.load_session,
        agent_name: report.agent_name,
        agent_version: report.agent_version,
        auth_methods: report.auth_methods,
        elapsed_ms: report.elapsed_ms,
    })
}

fn check_failure(category: &str, message: &str) -> CliEnvironmentCommandError {
    CliEnvironmentCommandError {
        category: category.to_string(),
        message: redact_text(message),
        retryable_with_a_new_plan: false,
        diagnostic_id: None,
    }
}
