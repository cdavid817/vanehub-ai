//! Running a CLI's own read-only commands, under bounds.
//!
//! Every probe is time-bounded, output-bounded, cancellable, and redacted *before* its text leaves
//! this module. Nothing downstream -- operation storage, the frontend DTO, the unified log -- ever
//! sees raw process output, so there is no path where redaction can be forgotten at one of three
//! call sites.
//!
//! Arguments are explicit argv. No probe is assembled into a shell string.

use std::time::Duration;

use crate::contexts::tooling::cli::application::environment_error::CliEnvironmentError;
use crate::contexts::tooling::cli::application::environment_ports::{
    CliCancellation, CliProbeOutcome, CliProbePort,
};
use crate::contexts::tooling::cli::domain::probe::CliProbeCommand;
use crate::platform::logging::redact_text;
use crate::platform::process::{ProcessAdapter, ProcessCancellation, ProcessError, ProcessRequest};

/// Appended once when output is cut. A single marker, so a reader can tell truncation from a
/// command that simply printed little.
pub(crate) const TRUNCATION_MARKER: &str = "[output truncated by VaneHub]";

pub(crate) struct SystemCliProbe;

impl CliProbePort for SystemCliProbe {
    fn run_probe(
        &self,
        executable_path: &str,
        command: CliProbeCommand,
        cancellation: &CliCancellation,
    ) -> Result<CliProbeOutcome, CliEnvironmentError> {
        // Checked before spawning: a cancelled operation must not start another child.
        if cancellation.is_cancelled() {
            return Ok(unstartable_outcome());
        }

        let request = ProcessRequest::new(executable_path)
            .args(command.args.iter().map(|arg| (*arg).to_string()))
            .timeout(Duration::from_secs(command.timeout_seconds))
            .output_limit(command.output_budget_bytes)
            .cancellation(ProcessCancellation::from_signal(cancellation.signal()));

        match ProcessAdapter.execute(&request) {
            Ok(output) => {
                let bounded =
                    bound_both_streams(&output.stdout, &output.stderr, command.output_budget_bytes);
                Ok(CliProbeOutcome {
                    exit_code: exit_code_of(&output.status_label()),
                    timed_out: false,
                    stdout: bounded.stdout,
                    stderr: bounded.stderr,
                    truncated: bounded.truncated,
                })
            }
            // A timeout is a probe result, not a failure of the refresh that requested it: the tool
            // is recorded as timing out and the scan continues to the next one. Whatever the child
            // printed before the deadline is kept -- it is often the useful part.
            Err(ProcessError::TimedOut {
                stdout,
                stderr,
                output_truncated,
                ..
            }) => {
                let bounded = bound_both_streams(&stdout, &stderr, command.output_budget_bytes);
                Ok(CliProbeOutcome {
                    exit_code: None,
                    timed_out: true,
                    stdout: bounded.stdout,
                    stderr: bounded.stderr,
                    truncated: output_truncated || bounded.truncated,
                })
            }
            Err(ProcessError::Cancelled { stdout, stderr, .. }) => {
                let bounded = bound_both_streams(&stdout, &stderr, command.output_budget_bytes);
                Ok(CliProbeOutcome {
                    exit_code: None,
                    timed_out: false,
                    stdout: bounded.stdout,
                    stderr: bounded.stderr,
                    truncated: bounded.truncated,
                })
            }
            // The executable could not be started -- deleted between discovery and probe, not
            // executable, wrong architecture. That is a fact about the *tool*, so it is a probe
            // result and the refresh continues to the next candidate.
            Err(ProcessError::InvalidExecutable(_) | ProcessError::Spawn(_)) => {
                Ok(unstartable_outcome())
            }
            // Anything else is VaneHub's own machinery misbehaving, which is worth surfacing.
            Err(other) => Err(CliEnvironmentError::Process(redact_text(
                &other.to_string(),
            ))),
        }
    }
}

/// Nothing ran and nothing was learned: cancelled before spawning, or the binary would not start.
fn unstartable_outcome() -> CliProbeOutcome {
    CliProbeOutcome {
        exit_code: None,
        timed_out: false,
        stdout: String::new(),
        stderr: String::new(),
        truncated: false,
    }
}

/// `ProcessOutput` reports a label rather than a raw code; zero is the only success.
fn exit_code_of(status_label: &str) -> Option<i32> {
    if status_label.eq_ignore_ascii_case("success") || status_label == "0" {
        return Some(0);
    }
    status_label
        .split_whitespace()
        .find_map(|token| token.parse::<i32>().ok())
        .or(Some(1))
}

fn exceeds_budget(value: &str, budget: usize) -> bool {
    value.len() > budget
}

/// Both streams of one probe, bounded under a single shared budget.
struct BoundedStreams {
    stdout: String,
    stderr: String,
    truncated: bool,
}

/// Applies one budget across stdout and stderr together.
///
/// A per-stream budget lets a command retain twice the intended volume by splitting its output, and
/// the documented ceiling for a doctor or auth probe is 128 KiB *total*. stdout is served first
/// because it carries the answer; stderr gets whatever is left, which may be nothing.
fn bound_both_streams(stdout: &str, stderr: &str, budget: usize) -> BoundedStreams {
    if exceeds_budget(stdout, budget) {
        // The budget is already spent and stdout carries the one marker. A second marker on stderr
        // would say truncation happened twice.
        return BoundedStreams {
            stdout: bound_and_redact(stdout, budget),
            stderr: String::new(),
            truncated: true,
        };
    }
    // What stdout did not consume.
    let remaining = budget - stdout.len();
    BoundedStreams {
        stdout: bound_and_redact(stdout, budget),
        stderr: bound_and_redact(stderr, remaining),
        truncated: exceeds_budget(stderr, remaining),
    }
}

/// Truncates on a character boundary, then redacts.
///
/// Redaction runs *after* truncation so a secret split across the cut cannot survive as two
/// halves that individually look harmless.
fn bound_and_redact(value: &str, budget: usize) -> String {
    let bounded = if value.len() > budget {
        let mut end = budget;
        while end > 0 && !value.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}{TRUNCATION_MARKER}", &value[..end])
    } else {
        value.to_string()
    };
    redact_text(&bounded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_within_budget_is_passed_through_redacted() {
        let bounded = bound_and_redact("claude-code 1.2.3", 1024);
        assert_eq!(bounded, "claude-code 1.2.3");
        assert!(!bounded.contains(TRUNCATION_MARKER));
    }

    #[test]
    fn output_over_budget_is_cut_once_with_a_single_marker() {
        let long = "a".repeat(5_000);
        let bounded = bound_and_redact(&long, 100);

        assert!(bounded.starts_with(&"a".repeat(100)));
        assert!(bounded.ends_with(TRUNCATION_MARKER));
        // Exactly one marker, so a reader can tell truncation happened once rather than repeatedly.
        assert_eq!(bounded.matches(TRUNCATION_MARKER).count(), 1);
    }

    #[test]
    fn truncation_never_splits_a_character() {
        // Multi-byte characters straddling the budget must not produce invalid UTF-8 or panic.
        let text = "版本 1.2.3 已安装".repeat(50);
        for budget in [1, 2, 3, 4, 5, 7, 11, 33] {
            let bounded = bound_and_redact(&text, budget);
            assert!(bounded.ends_with(TRUNCATION_MARKER), "budget {budget}");
        }
    }

    #[test]
    fn a_secret_in_probe_output_is_redacted_before_it_leaves_this_module() {
        let bounded = bound_and_redact("Authorization: Bearer sk-ant-secret-value-here", 4096);
        assert!(!bounded.contains("sk-ant-secret-value-here"));
    }

    #[test]
    fn a_secret_that_straddles_the_cut_cannot_survive_as_a_harmless_half() {
        // Redaction runs after truncation for exactly this case.
        let text = format!("prefix Authorization: Bearer {}", "s".repeat(200));
        let bounded = bound_and_redact(&text, 60);
        assert!(!bounded.contains(&"s".repeat(30)));
    }

    #[test]
    fn both_probe_streams_share_one_budget() {
        // 128 KiB total for a doctor probe, not 128 KiB each. A command that splits its output
        // across the two streams must not retain twice the ceiling.
        let bounded = bound_both_streams(&"a".repeat(60), &"b".repeat(60), 100);

        assert_eq!(bounded.stdout, "a".repeat(60));
        // Only the 40 bytes stdout left over.
        assert!(bounded.stderr.starts_with(&"b".repeat(40)));
        assert!(bounded.stderr.ends_with(TRUNCATION_MARKER));
        assert!(bounded.truncated);
        assert!(bounded.stdout.len() + bounded.stderr.len() < 100 + TRUNCATION_MARKER.len() * 2);
    }

    #[test]
    fn an_over_budget_stdout_leaves_no_second_marker_on_stderr() {
        let bounded = bound_both_streams(&"a".repeat(500), "warning: something", 100);

        assert!(bounded.stdout.ends_with(TRUNCATION_MARKER));
        assert_eq!(bounded.stdout.matches(TRUNCATION_MARKER).count(), 1);
        // Exactly one marker in the whole record.
        assert_eq!(bounded.stderr, "");
        assert!(bounded.truncated);
    }

    #[test]
    fn output_within_the_shared_budget_is_untouched_on_both_streams() {
        let bounded = bound_both_streams("claude-code 1.2.3", "npm warn deprecated", 4096);

        assert_eq!(bounded.stdout, "claude-code 1.2.3");
        assert_eq!(bounded.stderr, "npm warn deprecated");
        assert!(!bounded.truncated);
    }

    #[test]
    fn a_secret_on_stderr_is_redacted_under_the_shared_budget_too() {
        let bounded =
            bound_both_streams("ok", "Authorization: Bearer sk-ant-secret-value-here", 4096);
        assert!(!bounded.stderr.contains("sk-ant-secret-value-here"));
    }

    #[test]
    fn exceeds_budget_matches_the_truncation_decision() {
        assert!(!exceeds_budget("short", 100));
        assert!(exceeds_budget(&"x".repeat(101), 100));
        // The boundary itself is not over budget.
        assert!(!exceeds_budget(&"x".repeat(100), 100));
    }

    #[test]
    fn a_success_label_maps_to_a_zero_exit_code() {
        assert_eq!(exit_code_of("success"), Some(0));
        assert_eq!(exit_code_of("Success"), Some(0));
        assert_eq!(exit_code_of("0"), Some(0));
        // Anything else is a non-zero result, whatever its wording.
        assert_eq!(exit_code_of("exit code 3"), Some(3));
        assert_eq!(exit_code_of("terminated"), Some(1));
    }

    #[test]
    fn a_cancelled_probe_reports_nothing_rather_than_spawning() {
        let cancellation = CliCancellation::new(std::sync::Arc::new(
            std::sync::atomic::AtomicBool::new(true),
        ));
        let outcome = SystemCliProbe
            .run_probe(
                // A path that would fail loudly if it were ever executed.
                "/definitely/not/an/executable",
                CliProbeCommand::version(&["--version"]),
                &cancellation,
            )
            .expect("cancellation is a result, not an error");

        assert!(!outcome.succeeded());
        assert_eq!(outcome.exit_code, None);
        assert!(!outcome.timed_out);
        assert!(outcome.stdout.is_empty());
    }

    #[test]
    fn an_executable_that_cannot_be_started_is_a_result_not_an_error() {
        let outcome = SystemCliProbe
            .run_probe(
                "/definitely/not/an/executable",
                CliProbeCommand::version(&["--version"]),
                &CliCancellation::never(),
            )
            .expect("a missing binary is a probe result");

        // Recorded as not-runnable so the refresh can continue to the next installation.
        assert!(!outcome.succeeded());
        assert_eq!(outcome.exit_code, None);
    }

    #[test]
    fn probe_budgets_come_from_the_declared_command() {
        let version = CliProbeCommand::version(&["--version"]);
        let doctor = CliProbeCommand::diagnostic(&["doctor"], 30);
        assert_eq!(version.output_budget_bytes, 16 * 1024);
        assert_eq!(doctor.output_budget_bytes, 128 * 1024);
        assert_eq!(version.timeout_seconds, 10);
        assert_eq!(doctor.timeout_seconds, 30);
    }
}
