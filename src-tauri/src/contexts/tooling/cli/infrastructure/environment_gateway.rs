//! The one way a source adapter reaches a child process.
//!
//! Adapters depend on this trait rather than on `std::process`, so a test can assert the exact
//! argv an adapter would have run without a package manager being installed -- which is how "the
//! selected version reached the process arguments" becomes a fixture assertion instead of a
//! manual check.
//!
//! Every request is explicit argv. There is no field a shell string could be passed through.

use std::time::Duration;

use crate::contexts::tooling::cli::application::environment_error::CliEnvironmentError;
use crate::contexts::tooling::cli::application::environment_ports::{
    CliCancellation, CliOutputSink,
};
use crate::platform::logging::redact_text;
use crate::platform::process::{ProcessAdapter, ProcessCancellation, ProcessError, ProcessRequest};

use super::environment_probe::TRUNCATION_MARKER;

/// Retained output ceiling for one lifecycle operation.
pub(crate) const LIFECYCLE_OUTPUT_BUDGET_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliCommandRequest {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) timeout: Duration,
    /// Grouping label for the command audit trail. Never contains a version or a path.
    pub(crate) audit_category: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliCommandOutput {
    pub(crate) exit_code: Option<i32>,
    pub(crate) timed_out: bool,
    pub(crate) cancelled: bool,
    /// Already bounded and redacted.
    pub(crate) lines: Vec<String>,
    pub(crate) truncated: bool,
}

impl CliCommandOutput {
    pub(crate) fn succeeded(&self) -> bool {
        !self.timed_out && !self.cancelled && self.exit_code == Some(0)
    }

    pub(crate) fn joined(&self) -> String {
        self.lines.join("\n")
    }
}

pub(crate) trait CliCommandGateway: Send + Sync {
    fn run(
        &self,
        request: CliCommandRequest,
        cancellation: &CliCancellation,
        output: Option<&dyn CliOutputSink>,
    ) -> Result<CliCommandOutput, CliEnvironmentError>;
}

pub(crate) struct SystemCommandGateway;

impl CliCommandGateway for SystemCommandGateway {
    fn run(
        &self,
        request: CliCommandRequest,
        cancellation: &CliCancellation,
        output: Option<&dyn CliOutputSink>,
    ) -> Result<CliCommandOutput, CliEnvironmentError> {
        if cancellation.is_cancelled() {
            return Ok(CliCommandOutput {
                exit_code: None,
                timed_out: false,
                cancelled: true,
                lines: Vec::new(),
                truncated: false,
            });
        }
        crate::platform::process::audit_command(
            request.audit_category,
            &request.program,
            &request.args,
        );

        let process_request = ProcessRequest::new(&request.program)
            .args(request.args.clone())
            .timeout(request.timeout)
            .output_limit(LIFECYCLE_OUTPUT_BUDGET_BYTES)
            .cancellation(ProcessCancellation::from_signal(cancellation.signal()));

        let result = match ProcessAdapter.execute(&process_request) {
            Ok(process_output) => {
                let lines = bounded_lines(&process_output.stdout, &process_output.stderr);
                CliCommandOutput {
                    exit_code: if process_output.success() {
                        Some(0)
                    } else {
                        Some(1)
                    },
                    timed_out: false,
                    cancelled: false,
                    truncated: lines.iter().any(|line| line == TRUNCATION_MARKER),
                    lines,
                }
            }
            Err(ProcessError::TimedOut { stdout, stderr, .. }) => CliCommandOutput {
                exit_code: None,
                timed_out: true,
                cancelled: false,
                truncated: false,
                lines: bounded_lines(&stdout, &stderr),
            },
            Err(ProcessError::Cancelled { stdout, stderr, .. }) => CliCommandOutput {
                exit_code: None,
                timed_out: false,
                cancelled: true,
                truncated: false,
                lines: bounded_lines(&stdout, &stderr),
            },
            Err(other) => {
                return Err(CliEnvironmentError::Process(redact_text(
                    &other.to_string(),
                )))
            }
        };

        if let Some(sink) = output {
            for line in &result.lines {
                sink.emit(line);
            }
        }
        Ok(result)
    }
}

/// Merges both streams into one bounded, redacted, line-oriented record.
///
/// One budget across both streams rather than one each: a command that floods stderr must not be
/// able to retain twice the intended volume by splitting it.
fn bounded_lines(stdout: &str, stderr: &str) -> Vec<String> {
    let mut retained = Vec::new();
    let mut used = 0usize;
    for chunk in [stdout, stderr] {
        for line in chunk.lines() {
            if used >= LIFECYCLE_OUTPUT_BUDGET_BYTES {
                retained.push(TRUNCATION_MARKER.to_string());
                return retained;
            }
            let redacted = redact_text(line);
            used += redacted.len();
            retained.push(redacted);
        }
    }
    retained
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_streams_share_one_budget_and_get_one_marker() {
        let flood = "x".repeat(LIFECYCLE_OUTPUT_BUDGET_BYTES + 10);
        let lines = bounded_lines(&flood, "also here");

        assert_eq!(lines.last().map(String::as_str), Some(TRUNCATION_MARKER));
        // Exactly one marker: truncation happened, once.
        assert_eq!(
            lines
                .iter()
                .filter(|line| *line == TRUNCATION_MARKER)
                .count(),
            1
        );
        // The stderr content past the budget is not retained.
        assert!(!lines.iter().any(|line| line == "also here"));
    }

    #[test]
    fn ordinary_output_from_both_streams_is_kept_in_order() {
        let lines = bounded_lines("added 1 package\n", "npm warn deprecated\n");
        assert_eq!(lines, vec!["added 1 package", "npm warn deprecated"]);
    }

    #[test]
    fn output_is_redacted_before_it_is_retained() {
        let lines = bounded_lines("npm notice Authorization: Bearer sk-secret-value\n", "");
        assert!(!lines.join("\n").contains("sk-secret-value"));
    }

    #[test]
    fn a_command_only_succeeds_on_a_clean_zero_exit() {
        let base = CliCommandOutput {
            exit_code: Some(0),
            timed_out: false,
            cancelled: false,
            lines: Vec::new(),
            truncated: false,
        };
        assert!(base.succeeded());
        assert!(!CliCommandOutput {
            cancelled: true,
            ..base.clone()
        }
        .succeeded());
        assert!(!CliCommandOutput {
            timed_out: true,
            ..base.clone()
        }
        .succeeded());
        assert!(!CliCommandOutput {
            exit_code: Some(1),
            ..base
        }
        .succeeded());
    }

    #[test]
    fn a_cancelled_gateway_call_never_spawns() {
        let cancellation = CliCancellation::new(std::sync::Arc::new(
            std::sync::atomic::AtomicBool::new(true),
        ));
        let output = SystemCommandGateway
            .run(
                CliCommandRequest {
                    program: "/definitely/not/an/executable".to_string(),
                    args: vec!["--version".to_string()],
                    timeout: Duration::from_secs(5),
                    audit_category: "cli.test",
                },
                &cancellation,
                None,
            )
            .expect("cancellation is a result");

        assert!(output.cancelled);
        assert!(!output.succeeded());
        assert!(output.lines.is_empty());
    }

    #[test]
    fn joined_output_is_newline_separated() {
        let output = CliCommandOutput {
            exit_code: Some(0),
            timed_out: false,
            cancelled: false,
            lines: vec!["first".to_string(), "second".to_string()],
            truncated: false,
        };
        assert_eq!(output.joined(), "first\nsecond");
    }
}
