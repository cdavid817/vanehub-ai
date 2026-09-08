use tauri::State;

use super::background;
use super::dto::CliConnectionCheckDto;
use super::error::CliEnvironmentCommandError;
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::tooling::cli::api::CliApi;

/// An explicit, user-initiated ACP handshake with the installed program.
///
/// Separate from detection on purpose: detection runs only the read-only version probe, and a
/// handshake starts the vendor's agent. This is the one place that start is allowed to happen
/// from the management page, and it does `initialize` only -- no session, no prompt -- before
/// the process is released. The executable is the one detection resolved; a bare command name is
/// never re-resolved inside the child.
///
/// Async and off-thread: the handshake waits up to the ACP handshake budget for a program that
/// may be slow to start or never answer, and a synchronous command would hold the main thread
/// (window, navigation, every other IPC) for the whole of that wait.
#[tauri::command]
pub(crate) async fn check_cli_connection(
    api: State<'_, CliApi>,
    agent_runtime: State<'_, AgentRuntimeApi>,
    agent_id: String,
    provider_id: Option<String>,
) -> Result<CliConnectionCheckDto, CliEnvironmentCommandError> {
    let report = background::spawn_connection_check(
        api.inner().clone(),
        agent_runtime.inner().clone(),
        agent_id,
        provider_id,
    )
    .await
    .map_err(|error| background::check_failure("connection-check-failed", &error.to_string()))??;
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
