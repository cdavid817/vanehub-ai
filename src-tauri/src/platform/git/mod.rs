//! Git command adapter over the explicit process boundary.
use crate::platform::process::{ProcessAdapter, ProcessError, ProcessRequest};
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
        let request = ProcessRequest::new("git")
            .args(args.iter().cloned())
            .current_dir(root)
            .timeout(timeout);
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
