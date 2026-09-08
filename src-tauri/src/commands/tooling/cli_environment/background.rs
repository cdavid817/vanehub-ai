//! Where the variable-duration half of each command runs.
//!
//! The command returns an operation id the moment the work is queued. Every failure inside these
//! is already recorded on the operation by the service, so the join handle is dropped rather than
//! awaited -- there is nothing a caller could learn here that the operation does not already say.

use super::error::{command_error, CliEnvironmentCommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::agent_runtime::application::{
    ManagedConnectionCheckReport, ManagedConnectionCheckRequest,
};
use crate::contexts::tooling::cli::api::{CliApi, CliEnvironmentApi};
use crate::contexts::tooling::cli::application::environment_bulk::{
    PreparedCliBulkExecution, PreparedCliBulkPlanning, PreparedCliDoctor,
};
use crate::contexts::tooling::cli::application::environment_planning::{
    PreparedCliActionExecution, PreparedCliActionPlanning,
};
use crate::contexts::tooling::cli::application::environment_refresh::PreparedEnvironmentRefresh;
use crate::platform::logging::redact_text;

/// The ACP handshake of a connection check, off the main thread. Unlike the operations above it
/// has no operation record to report into, so the handle is returned for the command to await:
/// the page is waiting on this one answer.
pub(super) fn spawn_connection_check(
    api: CliApi,
    agent_runtime: AgentRuntimeApi,
    agent_id: String,
    provider_id: Option<String>,
) -> tauri::async_runtime::JoinHandle<
    Result<ManagedConnectionCheckReport, CliEnvironmentCommandError>,
> {
    tauri::async_runtime::spawn_blocking(move || {
        let executable = api
            .resolve_executable(&agent_id)
            .map_err(command_error)?
            .ok_or_else(|| check_failure("connection-check-not-installed", &agent_id))?;
        let workspace = std::env::temp_dir().to_string_lossy().to_string();
        agent_runtime
            .check_managed_connection(ManagedConnectionCheckRequest {
                agent_id: agent_id.clone(),
                executable,
                workspace,
                provider_id,
            })
            .map_err(|message| check_failure("connection-check-failed", &message))
    })
}

pub(super) fn check_failure(category: &str, message: &str) -> CliEnvironmentCommandError {
    CliEnvironmentCommandError {
        category: category.to_string(),
        message: redact_text(message),
        retryable_with_a_new_plan: false,
        diagnostic_id: None,
    }
}

pub(super) fn spawn_refresh(api: CliEnvironmentApi, prepared: PreparedEnvironmentRefresh) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = api.execute_refresh(prepared);
    });
}

pub(super) fn spawn_action_planning(api: CliEnvironmentApi, prepared: PreparedCliActionPlanning) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = api.execute_action_planning(prepared);
    });
}

pub(super) fn spawn_action(api: CliEnvironmentApi, prepared: PreparedCliActionExecution) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = api.execute_action(prepared);
    });
}

pub(super) fn spawn_bulk_planning(api: CliEnvironmentApi, prepared: PreparedCliBulkPlanning) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = api.execute_bulk_planning(prepared);
    });
}

pub(super) fn spawn_bulk_action(api: CliEnvironmentApi, prepared: PreparedCliBulkExecution) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = api.execute_bulk_action(prepared);
    });
}

pub(super) fn spawn_doctor(api: CliEnvironmentApi, prepared: PreparedCliDoctor) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = api.execute_doctor(prepared);
    });
}
