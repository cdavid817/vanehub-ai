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

impl GitAdapter {
    pub(crate) fn execute(
        &self,
        root: &Path,
        args: &[String],
        timeout: Duration,
    ) -> Result<GitOutput, ProcessError> {
        let request = base_request(root, args, timeout);
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
        let mut request = base_request(root, args, timeout);
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

    /// Runs Git with the inherited repository-selecting variables dropped.
    ///
    /// `GIT_DIR`, `GIT_WORK_TREE` and friends redirect *every* Git command to another repository,
    /// so a probe that inherited them from a shell would inspect — or remove — something other
    /// than the directory it was pointed at. Optional locks and the fsmonitor hook are disabled
    /// for the same reason a probe must be read-only: neither may write into the target or start
    /// a program from it.
    pub(crate) fn execute_isolated(
        &self,
        root: &Path,
        args: &[String],
        timeout: Duration,
    ) -> Result<GitOutput, ProcessError> {
        let mut request = base_request(root, &[], timeout)
            .arg("--no-optional-locks")
            .arg("-c")
            .arg("core.fsmonitor=false")
            .args(args.iter().cloned());
        for key in REPOSITORY_SELECTING_VARIABLES {
            request = request.env_remove(key);
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

/// Inherited variables that would make Git act on a repository other than `root`.
const REPOSITORY_SELECTING_VARIABLES: [&str; 7] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_COMMON_DIR",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_NAMESPACE",
];

/// `LC_ALL=C` pins git's message language: callers classify outcomes by matching output text
/// ("not a git repository", "did not match any files"), and on a zh_CN host git otherwise
/// localizes those messages, silently breaking every such match
/// (`session-project-inspection`'s "Git inspection outcomes are locale-independent"). Applied
/// before caller-supplied environment so an explicit override still wins.
fn base_request(root: &Path, args: &[String], timeout: Duration) -> ProcessRequest {
    ProcessRequest::new("git")
        .args(args.iter().cloned())
        .current_dir(root)
        .env("LC_ALL", "C")
        .timeout(timeout)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn temp_non_git_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vanehub-git-locale-test-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn execute_yields_english_classifiable_messages_regardless_of_host_locale() {
        let dir = temp_non_git_dir("execute");
        let output = GitAdapter::default()
            .execute(&dir, &["status".to_string()], Duration::from_secs(10))
            .expect("git should run");
        let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
        assert!(!output.status.success());
        assert!(
            stderr.contains("not a git repository"),
            "expected the English classification marker, got: {stderr}"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn pinned_locale_beats_a_non_english_caller_language_environment() {
        let dir = temp_non_git_dir("with-env");
        // LANG/LC_MESSAGES/LANGUAGE sit below LC_ALL in libc's precedence, so the pinned
        // LC_ALL=C must win over all of them.
        let environment: BTreeMap<String, String> = [
            ("LANG", "zh_CN.UTF-8"),
            ("LC_MESSAGES", "zh_CN.UTF-8"),
            ("LANGUAGE", "zh_CN"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
        let output = GitAdapter::default()
            .execute_with_environment(
                &dir,
                &["status".to_string()],
                &environment,
                Duration::from_secs(10),
            )
            .expect("git should run");
        let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
        assert!(!output.status.success());
        assert!(
            stderr.contains("not a git repository"),
            "expected the English classification marker, got: {stderr}"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_explicit_caller_lc_all_overrides_the_pinned_default() {
        let request = base_request(
            Path::new("."),
            &["status".to_string()],
            Duration::from_secs(1),
        )
        .env("LC_ALL", "zh_CN.UTF-8");
        let command = request.command().expect("command should build");
        let lc_all = command
            .get_envs()
            .find(|(key, _)| *key == std::ffi::OsStr::new("LC_ALL"))
            .and_then(|(_, value)| value.map(std::ffi::OsStr::to_os_string));
        assert_eq!(lc_all.as_deref(), Some(std::ffi::OsStr::new("zh_CN.UTF-8")));
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
