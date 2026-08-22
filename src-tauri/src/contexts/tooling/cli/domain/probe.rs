//! Which read-only commands may be run against a CLI, and what may be concluded from them.
//!
//! Probes are backend data, not adapter conditionals. Every one is non-interactive, bounded in
//! time and output, and never prompts for or captures a credential.
//!
//! The rule that shapes this file: an absent or unverified probe yields `Unknown`. It never yields
//! `authenticated` or `ready`. Reporting a CLI as logged in because no probe contradicted it is
//! worse than admitting VaneHub cannot tell.

/// A bounded, read-only invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CliProbeCommand {
    /// Arguments appended to the tool's own executable. Never a shell string.
    pub(crate) args: &'static [&'static str],
    pub(crate) timeout_seconds: u64,
    /// Retained output ceiling per stream. Exceeding it truncates with a single marker; the child
    /// keeps being drained so it cannot deadlock on a full pipe.
    pub(crate) output_budget_bytes: usize,
}

impl CliProbeCommand {
    /// Version probes are the cheapest and most frequent; a small budget is plenty for a version
    /// string and stops a misbehaving binary from streaming megabytes into a refresh.
    pub(crate) const VERSION_BUDGET_BYTES: usize = 16 * 1024;
    /// Doctor and auth probes print structured diagnostics, so they get more room -- but a total,
    /// not per stream, because both streams are interleaved into one bounded record.
    pub(crate) const DIAGNOSTIC_BUDGET_BYTES: usize = 128 * 1024;

    pub(crate) const fn version(args: &'static [&'static str]) -> Self {
        Self {
            args,
            timeout_seconds: 10,
            output_budget_bytes: Self::VERSION_BUDGET_BYTES,
        }
    }

    pub(crate) const fn diagnostic(args: &'static [&'static str], timeout_seconds: u64) -> Self {
        Self {
            args,
            timeout_seconds,
            output_budget_bytes: Self::DIAGNOSTIC_BUDGET_BYTES,
        }
    }
}

/// Whether a probe exists for a given readiness question, and why not when it does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliProbeAvailability {
    Supported(CliProbeCommand),
    /// No documented non-interactive command exists, or the one that exists has no stable signal
    /// worth parsing. The derived state is `Unknown`, and that is a truthful answer.
    Undocumented,
}

impl CliProbeAvailability {
    pub(crate) fn command(self) -> Option<CliProbeCommand> {
        match self {
            Self::Supported(command) => Some(command),
            Self::Undocumented => None,
        }
    }

    pub(crate) fn is_supported(self) -> bool {
        matches!(self, Self::Supported(_))
    }
}

/// The complete probe surface for one CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CliProbeDefinition {
    /// Every registered CLI reports a version; this one is always present.
    pub(crate) version: CliProbeCommand,
    pub(crate) doctor: CliProbeAvailability,
    pub(crate) authentication: CliProbeAvailability,
}

impl CliProbeDefinition {
    /// A CLI whose only verified read-only command is `--version`.
    pub(crate) const fn version_only() -> Self {
        Self {
            version: CliProbeCommand::version(&["--version"]),
            doctor: CliProbeAvailability::Undocumented,
            authentication: CliProbeAvailability::Undocumented,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_only_cli_reports_unknown_rather_than_ready() {
        let definition = CliProbeDefinition::version_only();
        assert_eq!(definition.version.args, &["--version"]);
        assert!(!definition.doctor.is_supported());
        assert!(!definition.authentication.is_supported());
        assert_eq!(definition.doctor.command(), None);
        assert_eq!(definition.authentication.command(), None);
    }

    #[test]
    fn probe_budgets_separate_version_output_from_diagnostics() {
        let version = CliProbeCommand::version(&["--version"]);
        let doctor = CliProbeCommand::diagnostic(&["doctor"], 30);

        assert_eq!(version.output_budget_bytes, 16 * 1024);
        assert_eq!(doctor.output_budget_bytes, 128 * 1024);
        assert!(doctor.timeout_seconds > version.timeout_seconds);
    }

    #[test]
    fn every_probe_is_bounded_in_time_and_output() {
        let commands = [
            CliProbeCommand::version(&["--version"]),
            CliProbeCommand::diagnostic(&["doctor"], 30),
            CliProbeCommand::diagnostic(&["login", "status"], 20),
        ];
        for command in commands {
            assert!(command.timeout_seconds > 0);
            assert!(command.output_budget_bytes > 0);
            // Explicit argv, never a shell string that could carry an operator.
            assert!(command.args.iter().all(|arg| !arg.contains('|')));
            assert!(command.args.iter().all(|arg| !arg.contains(';')));
        }
    }
}
