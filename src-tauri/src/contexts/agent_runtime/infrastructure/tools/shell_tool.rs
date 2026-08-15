use super::ToolExecutionOutcome;
use super::MAX_TOOL_OUTPUT_BYTES;
use crate::platform::process::{ProcessAdapter, ProcessCancellation, ProcessError, ProcessRequest};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

/// The foreground timeout applied when a call supplies no explicit `timeout_ms`. Unchanged from
/// before `add-background-shell-execution`, so an existing call's behavior is bit-for-bit the same.
pub(crate) const DEFAULT_SHELL_TIMEOUT_MS: u64 = 60_000;

/// The ceiling `timeout_ms` is clamped to. A caller may lower its budget but never raise it past
/// this, matching how `file`'s `limit` and grep's `head_limit` already behave. Work that needs
/// longer than this belongs in background mode, which is bounded by lifetime instead.
pub(crate) const MAX_SHELL_TIMEOUT_MS: u64 = 600_000;

pub(super) fn shell_invocation() -> (&'static str, &'static str) {
    if cfg!(target_os = "windows") {
        ("cmd", "/C")
    } else {
        ("bash", "-c")
    }
}

/// Clamps a model-supplied timeout into `[1, MAX_SHELL_TIMEOUT_MS]`, falling back to the default
/// when absent. Clamping rather than rejecting keeps an over-eager value from failing a call the
/// user already approved.
pub(crate) fn effective_shell_timeout_ms(requested: Option<u64>) -> u64 {
    requested.map_or(DEFAULT_SHELL_TIMEOUT_MS, |value| {
        value.clamp(1, MAX_SHELL_TIMEOUT_MS)
    })
}

/// Executes `command` in `workspace_folder` through `platform::process`'s bounded, timed-out,
/// cancellable subprocess execution. `command` is passed as a single explicit argument to the
/// platform shell (never concatenated into a larger command line), matching how the shell tool
/// is declared to the model: the shell itself, not VaneHub, interprets `command`'s syntax.
pub(crate) fn execute_shell(
    command: &str,
    workspace_folder: &str,
    cancelled: Arc<AtomicBool>,
    timeout_ms: Option<u64>,
) -> ToolExecutionOutcome {
    let timeout = Duration::from_millis(effective_shell_timeout_ms(timeout_ms));
    let (executable, flag) = shell_invocation();
    crate::platform::process::audit_command(
        "agent_runtime.tool.shell",
        executable,
        &[flag.to_string(), command.to_string()],
    );
    let request = ProcessRequest::new(executable)
        .arg(flag)
        .arg(command)
        .current_dir(workspace_folder)
        .timeout(timeout)
        .cancellation(ProcessCancellation::from_signal(cancelled))
        .output_limit(MAX_TOOL_OUTPUT_BYTES);
    match ProcessAdapter.execute(&request) {
        Ok(output) => {
            let success = output.success();
            let combined = if output.stderr.is_empty() {
                output.stdout
            } else if output.stdout.is_empty() {
                output.stderr
            } else {
                format!("{}\n{}", output.stdout, output.stderr)
            };
            ToolExecutionOutcome {
                output: combined,
                is_error: !success,
            }
        }
        Err(ProcessError::TimedOut { stdout, stderr, .. }) => ToolExecutionOutcome {
            output: format!(
                "Command timed out after {}s. Long-running work belongs in background mode \
                 (run_in_background), which is bounded by lifetime instead of this \
                 timeout.\nstdout: {stdout}\nstderr: {stderr}",
                timeout.as_secs()
            ),
            is_error: true,
        },
        Err(ProcessError::Cancelled { .. }) => ToolExecutionOutcome {
            output: "Command was cancelled.".to_string(),
            is_error: true,
        },
        Err(error) => ToolExecutionOutcome {
            output: error.to_string(),
            is_error: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn not_cancelled() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    #[test]
    fn runs_a_command_and_captures_stdout() {
        let outcome = execute_shell("echo hello-from-shell-tool", ".", not_cancelled(), None);
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("hello-from-shell-tool"));
    }

    #[test]
    fn a_failing_command_is_reported_as_an_error_without_terminating() {
        let failing_command = if cfg!(target_os = "windows") {
            "exit /b 1"
        } else {
            "exit 1"
        };
        let outcome = execute_shell(failing_command, ".", not_cancelled(), None);
        assert!(outcome.is_error);
    }

    #[test]
    fn cancellation_before_completion_is_reported_as_an_error() {
        let cancelled = Arc::new(AtomicBool::new(true));
        let long_running = if cfg!(target_os = "windows") {
            "ping -n 6 127.0.0.1"
        } else {
            "sleep 5"
        };
        let outcome = execute_shell(long_running, ".", cancelled, None);
        assert!(outcome.is_error);
    }

    #[test]
    fn an_absent_timeout_keeps_the_previous_fixed_budget() {
        assert_eq!(effective_shell_timeout_ms(None), DEFAULT_SHELL_TIMEOUT_MS);
    }

    #[test]
    fn a_supplied_timeout_may_lower_the_budget_but_never_raise_it_past_the_ceiling() {
        assert_eq!(effective_shell_timeout_ms(Some(5_000)), 5_000);
        assert_eq!(
            effective_shell_timeout_ms(Some(MAX_SHELL_TIMEOUT_MS + 1)),
            MAX_SHELL_TIMEOUT_MS
        );
        assert_eq!(
            effective_shell_timeout_ms(Some(u64::MAX)),
            MAX_SHELL_TIMEOUT_MS
        );
        // Zero would otherwise mean "time out before the process is even spawned".
        assert_eq!(effective_shell_timeout_ms(Some(0)), 1);
    }

    #[test]
    fn a_short_explicit_timeout_terminates_a_long_command() {
        let long_running = if cfg!(target_os = "windows") {
            "ping -n 20 127.0.0.1"
        } else {
            "sleep 20"
        };
        let outcome = execute_shell(long_running, ".", not_cancelled(), Some(500));
        assert!(outcome.is_error);
        assert!(
            outcome.output.contains("timed out"),
            "expected a timeout report, got {:?}",
            outcome.output
        );
    }
}
