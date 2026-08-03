#[path = "runtime_logging_model.rs"]
mod model;

use crate::contexts::tooling::mcp::application::McpRuntimeError;
use crate::contexts::tooling::mcp::domain::McpFailureCode;
use crate::platform::logging::{self, LogLevel};
pub(super) use model::McpRuntimeLogContext;
use model::{RuntimeDiagnostic, RuntimeOutcome, RuntimePhase};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

const CATEGORY: &str = "mcp.runtime";
const EMERGENCY_CATEGORY: &str = "mcp.runtime.emergency";

pub(super) fn record_command_start(context: &McpRuntimeLogContext) {
    persist(&RuntimeDiagnostic {
        context,
        phase: RuntimePhase::Command,
        outcome: RuntimeOutcome::Started,
        error_code: None,
        duration: None,
        stderr_observed_bytes: None,
        stderr_truncated: None,
        exit_code: None,
    });
}

pub(super) fn record_success(context: &McpRuntimeLogContext, duration: Duration) {
    record_terminal(context, None, RuntimeOutcome::Succeeded, duration);
}

pub(super) fn record_error(
    context: &McpRuntimeLogContext,
    error: &McpRuntimeError,
    duration: Duration,
) {
    let outcome = match error.code() {
        McpFailureCode::Timeout => RuntimeOutcome::TimedOut,
        McpFailureCode::Cancelled => RuntimeOutcome::Cancelled,
        _ => RuntimeOutcome::Failed,
    };
    record_terminal(context, Some(error.code()), outcome, duration);
}

fn record_terminal(
    context: &McpRuntimeLogContext,
    error_code: Option<McpFailureCode>,
    outcome: RuntimeOutcome,
    duration: Duration,
) {
    let phase = match error_code {
        Some(McpFailureCode::Spawn) => RuntimePhase::Spawn,
        Some(McpFailureCode::Cleanup) => RuntimePhase::Cleanup,
        Some(
            McpFailureCode::Protocol | McpFailureCode::Transport | McpFailureCode::UpstreamHttp,
        ) => RuntimePhase::Protocol,
        _ => RuntimePhase::Protocol,
    };
    persist(&RuntimeDiagnostic {
        context,
        phase,
        outcome,
        error_code,
        duration: Some(duration),
        stderr_observed_bytes: None,
        stderr_truncated: None,
        exit_code: None,
    });
}

pub(super) fn record_child_exit(
    context: &McpRuntimeLogContext,
    exit_code: Option<i32>,
    stderr_observed_bytes: u64,
    stderr_truncated: bool,
) {
    persist(&RuntimeDiagnostic {
        context,
        phase: RuntimePhase::Exit,
        outcome: if exit_code == Some(0) {
            RuntimeOutcome::Succeeded
        } else {
            RuntimeOutcome::Terminated
        },
        error_code: None,
        duration: None,
        stderr_observed_bytes: Some(stderr_observed_bytes),
        stderr_truncated: Some(stderr_truncated),
        exit_code,
    });
}

pub(super) fn record_relay_terminal(
    context: &McpRuntimeLogContext,
    error_code: Option<McpFailureCode>,
) {
    persist(&RuntimeDiagnostic {
        context,
        phase: RuntimePhase::Relay,
        outcome: if error_code.is_none() {
            RuntimeOutcome::Succeeded
        } else {
            RuntimeOutcome::Failed
        },
        error_code,
        duration: None,
        stderr_observed_bytes: None,
        stderr_truncated: None,
        exit_code: None,
    });
}

#[cfg(not(test))]
fn persist(diagnostic: &RuntimeDiagnostic<'_>) {
    let fallback = logging::fallback_log_dir();
    let active = logging::active_log_dir(fallback.clone());
    persist_to_dirs(diagnostic, &active, &fallback);
}

#[cfg(test)]
fn persist(_diagnostic: &RuntimeDiagnostic<'_>) {}

fn persist_to_dirs(diagnostic: &RuntimeDiagnostic<'_>, normal: &Path, emergency: &Path) {
    let (level, message, context) = model::render(diagnostic);
    if logging::write_message(normal, level, CATEGORY, message, context).is_err() {
        let context = BTreeMap::from([(
            "classification".to_string(),
            "mcp_runtime_logging_unavailable".to_string(),
        )]);
        let _ = logging::write_message(
            emergency,
            LogLevel::Error,
            EMERGENCY_CATEGORY,
            "MCP runtime diagnostic persistence failed",
            context,
        );
    }
}

#[cfg(test)]
#[path = "runtime_logging_tests.rs"]
mod tests;
