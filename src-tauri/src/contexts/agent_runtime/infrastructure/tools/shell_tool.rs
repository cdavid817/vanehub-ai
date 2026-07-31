use super::ToolExecutionOutcome;
use crate::platform::process::{ProcessAdapter, ProcessCancellation, ProcessError, ProcessRequest};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

const SHELL_TIMEOUT: Duration = Duration::from_secs(60);
const SHELL_OUTPUT_LIMIT: usize = 64 * 1024;

fn shell_invocation() -> (&'static str, &'static str) {
    if cfg!(target_os = "windows") {
        ("cmd", "/C")
    } else {
        ("bash", "-c")
    }
}

/// Executes `command` in `workspace_folder` through `platform::process`'s bounded, timed-out,
/// cancellable subprocess execution. `command` is passed as a single explicit argument to the
/// platform shell (never concatenated into a larger command line), matching how the shell tool
/// is declared to the model: the shell itself, not VaneHub, interprets `command`'s syntax.
pub(crate) fn execute_shell(
    command: &str,
    workspace_folder: &str,
    cancelled: Arc<AtomicBool>,
) -> ToolExecutionOutcome {
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
        .timeout(SHELL_TIMEOUT)
        .cancellation(ProcessCancellation::from_signal(cancelled))
        .output_limit(SHELL_OUTPUT_LIMIT);
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
                "Command timed out after {}s.\nstdout: {stdout}\nstderr: {stderr}",
                SHELL_TIMEOUT.as_secs()
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
        let outcome = execute_shell("echo hello-from-shell-tool", ".", not_cancelled());
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
        let outcome = execute_shell(failing_command, ".", not_cancelled());
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
        let outcome = execute_shell(long_running, ".", cancelled);
        assert!(outcome.is_error);
    }
}
