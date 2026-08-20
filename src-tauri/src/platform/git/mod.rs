//! Git command adapter over the explicit process boundary.
use crate::platform::process::{ProcessAdapter, ProcessError, ProcessRequest};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitStatus;
use std::time::Duration;

#[derive(Debug)]
pub(crate) struct GitOutput {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct GitAdapter {
    process: ProcessAdapter,
}

/// Callers classify git failures by matching its stderr -- "not a git repository" is the
/// difference between "this folder is not a project" and "git is broken", and the first is a
/// normal answer the create-session dialog renders as `Folder`. Those messages are translated, so
/// on a machine whose git speaks anything but English every one of those matches silently misses
/// and a normal answer is reported as a launch failure.
///
/// `LC_ALL=C` pins git's messages to English for the parsing to work against. gettext ignores
/// `LANGUAGE` once the locale is `C`, but it is cleared as well because `LANGUAGE` takes
/// precedence over `LC_ALL` for any git build that resolves messages before that rule applies.
/// Paths are unaffected: callers that care already pass `core.quotepath=false`, and git writes
/// path bytes through regardless of locale.
fn with_stable_message_locale(request: ProcessRequest) -> ProcessRequest {
    request.env("LC_ALL", "C").env("LANGUAGE", "")
}

impl GitAdapter {
    pub(crate) fn execute(
        &self,
        root: &Path,
        args: &[String],
        timeout: Duration,
    ) -> Result<GitOutput, ProcessError> {
        let request = with_stable_message_locale(
            ProcessRequest::new("git")
                .args(args.iter().cloned())
                .current_dir(root)
                .timeout(timeout),
        );
        let output = self.process.execute(&request)?;
        Ok(GitOutput {
            status: output.status,
            stdout: output.stdout_bytes,
            stderr: output.stderr_bytes,
        })
    }

    #[allow(dead_code)]
    pub(crate) fn execute_with_environment(
        &self,
        root: &Path,
        args: &[String],
        environment: &BTreeMap<String, String>,
        timeout: Duration,
    ) -> Result<GitOutput, ProcessError> {
        // The locale is applied first so an explicit caller environment still wins, which keeps
        // this from overriding a caller that deliberately sets its own.
        let mut request = with_stable_message_locale(
            ProcessRequest::new("git")
                .args(args.iter().cloned())
                .current_dir(root)
                .timeout(timeout),
        );
        for (key, value) in environment {
            request = request.env(key, value);
        }
        let output = self.process.execute(&request)?;
        Ok(GitOutput {
            status: output.status,
            stdout: output.stdout_bytes,
            stderr: output.stderr_bytes,
        })
    }

    pub(crate) fn redacted_diagnostic(operation: &str, root: &Path, output: &GitOutput) -> String {
        let raw = format!(
            "git {operation} status={} stderr={}",
            output
                .status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| output.status.to_string()),
            String::from_utf8_lossy(&output.stderr)
        );
        let root = root.to_string_lossy();
        let without_workspace = raw.replace(root.as_ref(), "[WORKSPACE]");
        crate::platform::logging::redact_text(&without_workspace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the locale pinning itself. Every other test of this behaviour only fails on a machine
    /// whose git is translated -- `git_fixtures_cover_non_git_and_common_worktree_states` sat green
    /// on an English CI runner while reporting `LaunchFailed` for a plain folder on a Chinese one --
    /// so the guarantee is asserted here against the request, where it holds on any host.
    #[test]
    fn git_runs_under_a_pinned_message_locale_so_stderr_matching_survives_translation() {
        let request = with_stable_message_locale(ProcessRequest::new("git"));

        assert_eq!(
            request
                .environment_value("LC_ALL")
                .map(|value| value.to_string_lossy().into_owned()),
            Some("C".to_string()),
            "git was not pinned to the C message locale",
        );
        assert_eq!(
            request
                .environment_value("LANGUAGE")
                .map(|value| value.to_string_lossy().into_owned()),
            Some(String::new()),
            "LANGUAGE was not cleared, and it outranks LC_ALL for gettext",
        );
    }

    #[test]
    fn diagnostics_hide_workspace_paths_and_credentials() {
        let status = success_status();
        let root = Path::new("C:\\Users\\private-user\\workspace");
        let output = GitOutput {
            status,
            stdout: Vec::new(),
            stderr: b"C:\\Users\\private-user\\workspace token=git-secret".to_vec(),
        };

        let diagnostic = GitAdapter::redacted_diagnostic("status", root, &output);

        assert!(diagnostic.contains("[WORKSPACE]"));
        assert!(diagnostic.contains("token=[REDACTED]"));
        assert!(!diagnostic.contains("private-user"));
        assert!(!diagnostic.contains("git-secret"));
    }

    #[cfg(windows)]
    fn success_status() -> ExitStatus {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(0)
    }

    #[cfg(unix)]
    fn success_status() -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(0)
    }
}
