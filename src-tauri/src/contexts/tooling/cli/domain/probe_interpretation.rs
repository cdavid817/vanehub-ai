//! Turning a bounded probe's output into a normalized state.
//!
//! Pure functions over already-redacted text. They return an enum and nothing else -- no substring
//! of the output ever escapes, so a token that survived redaction cannot leak through a state
//! value.
//!
//! The rule every parser obeys: **silence is never consent**. A command that did not run, timed
//! out, was cancelled, or printed something unrecognised yields `Unknown`. Only an explicit,
//! documented signal yields `Authenticated` or `Ok`. Reporting a CLI as logged in because nothing
//! contradicted it is worse than admitting VaneHub cannot tell.

use super::status::CliAuthenticationStatus;

/// What a Doctor probe concluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliDoctorVerdict {
    Ok,
    Problem,
    /// No probe, no answer, or an answer nobody can interpret.
    Unknown,
}

impl CliDoctorVerdict {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Problem => "problem",
            Self::Unknown => "unknown",
        }
    }

    pub(crate) fn reports_problem(self) -> bool {
        matches!(self, Self::Problem)
    }
}

/// How a tool's authentication output is read. Selected by the registry, never guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliAuthParser {
    /// No documented non-interactive probe. Always `Unknown`.
    Undocumented,
    /// `codex login status`: a documented account line, or a documented not-logged-in line.
    CodexLoginStatus,
    /// `opencode auth list`: reduced to whether any credential entry exists. The entries
    /// themselves are never read past counting them.
    OpenCodeAuthList,
}

/// How a tool's Doctor output is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliDoctorParser {
    Undocumented,
    /// `claude doctor`: exit status, with unsupported-command detection.
    ClaudeCodeDoctor,
}

/// The bounded facts a parser is allowed to see.
///
/// Deliberately not the full `CliProbeOutcome`: a parser gets exit status, the two failure flags,
/// and text that has already been truncated and redacted upstream.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CliProbeReading<'a> {
    pub(crate) exit_code: Option<i32>,
    pub(crate) timed_out: bool,
    pub(crate) cancelled: bool,
    pub(crate) stdout: &'a str,
    pub(crate) stderr: &'a str,
}

impl CliProbeReading<'_> {
    fn succeeded(&self) -> bool {
        !self.timed_out && !self.cancelled && self.exit_code == Some(0)
    }

    /// Neither stream said anything a parser could act on.
    fn ran_at_all(&self) -> bool {
        !self.timed_out && !self.cancelled && self.exit_code.is_some()
    }

    fn combined_lowercase(&self) -> String {
        format!("{} {}", self.stdout, self.stderr).to_ascii_lowercase()
    }
}

/// Phrases every CLI in this catalog uses when a subcommand does not exist.
///
/// Matters because "the command is not supported here" and "you are not logged in" produce the
/// same non-zero exit, and treating the first as the second would tell the user to log in to a
/// tool that has no login.
const UNSUPPORTED_COMMAND_MARKERS: [&str; 6] = [
    "unknown command",
    "unrecognized subcommand",
    "unrecognised subcommand",
    "no such command",
    "invalid choice",
    "is not a",
];

const EXPIRED_MARKERS: [&str; 4] = [
    "expired",
    "token is invalid",
    "reauthenticate",
    "re-authenticate",
];

const NOT_AUTHENTICATED_MARKERS: [&str; 5] = [
    "not logged in",
    "no credentials",
    "not authenticated",
    "please log in",
    "please login",
];

const AUTHENTICATED_MARKERS: [&str; 4] = ["logged in", "authenticated as", "signed in", "account:"];

fn mentions(haystack: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| haystack.contains(marker))
}

/// Interprets an authentication probe.
pub(crate) fn interpret_authentication(
    parser: CliAuthParser,
    reading: CliProbeReading<'_>,
) -> CliAuthenticationStatus {
    if parser == CliAuthParser::Undocumented || !reading.ran_at_all() {
        // No probe, a timeout, or a cancellation. None of those is evidence either way.
        return CliAuthenticationStatus::Unknown;
    }
    let text = reading.combined_lowercase();
    if mentions(&text, &UNSUPPORTED_COMMAND_MARKERS) {
        // The build in front of us does not have this subcommand. That says nothing about login.
        return CliAuthenticationStatus::Unknown;
    }
    // Expiry is checked before the not-logged-in phrases, because an expired session usually
    // reports both and "expired" is the more actionable of the two.
    if mentions(&text, &EXPIRED_MARKERS) {
        return CliAuthenticationStatus::Expired;
    }
    if mentions(&text, &NOT_AUTHENTICATED_MARKERS) {
        return CliAuthenticationStatus::Required;
    }

    match parser {
        CliAuthParser::Undocumented => CliAuthenticationStatus::Unknown,
        CliAuthParser::CodexLoginStatus => {
            if reading.succeeded() && mentions(&text, &AUTHENTICATED_MARKERS) {
                CliAuthenticationStatus::Authenticated
            } else if reading.succeeded() {
                // Exit zero with nothing recognisable. Not a licence to claim a session.
                CliAuthenticationStatus::Unknown
            } else {
                CliAuthenticationStatus::Required
            }
        }
        CliAuthParser::OpenCodeAuthList => {
            if !reading.succeeded() {
                return CliAuthenticationStatus::Unknown;
            }
            // Reduced to a count. The entries name providers and accounts, and none of that has
            // any business reaching a state value.
            if credential_entry_count(reading.stdout) > 0 {
                CliAuthenticationStatus::Authenticated
            } else {
                CliAuthenticationStatus::Required
            }
        }
    }
}

/// Counts credential rows in an `auth list` table, ignoring headers and rules.
fn credential_entry_count(stdout: &str) -> usize {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with('-') && !line.starts_with('='))
        .filter(|line| {
            let lowered = line.to_ascii_lowercase();
            // Drop a header row and the "nothing here" line some builds print.
            !lowered.starts_with("provider")
                && !lowered.starts_with("name")
                && !lowered.contains("no credentials")
                && !lowered.contains("nothing")
        })
        .count()
}

/// Interprets a Doctor probe.
pub(crate) fn interpret_doctor(
    parser: CliDoctorParser,
    reading: CliProbeReading<'_>,
) -> CliDoctorVerdict {
    if parser == CliDoctorParser::Undocumented || !reading.ran_at_all() {
        return CliDoctorVerdict::Unknown;
    }
    let text = reading.combined_lowercase();
    if mentions(&text, &UNSUPPORTED_COMMAND_MARKERS) {
        // An older build without `doctor` is not a broken installation.
        return CliDoctorVerdict::Unknown;
    }
    if reading.succeeded() {
        CliDoctorVerdict::Ok
    } else {
        CliDoctorVerdict::Problem
    }
}

#[cfg(test)]
#[path = "probe_interpretation_tests.rs"]
mod tests;
