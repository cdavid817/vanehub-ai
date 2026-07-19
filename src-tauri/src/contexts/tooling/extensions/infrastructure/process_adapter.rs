use crate::platform::process::{ProcessAdapter, ProcessError, ProcessRequest};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExtensionProcessRequest {
    pub(super) executable: String,
    pub(super) args: Vec<String>,
    pub(super) current_dir: Option<PathBuf>,
    pub(super) timeout: Duration,
    pub(super) audit_category: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExtensionProcessOutput {
    pub(super) success: bool,
    pub(super) stdout: String,
    pub(super) stderr: String,
    pub(super) status: String,
}

pub(super) trait ExtensionProcessRunner: Send + Sync {
    fn execute(&self, request: ExtensionProcessRequest) -> Result<ExtensionProcessOutput, String>;
}

#[derive(Debug, Clone, Copy, Default)]
struct PlatformExtensionProcessRunner;

impl ExtensionProcessRunner for PlatformExtensionProcessRunner {
    fn execute(&self, request: ExtensionProcessRequest) -> Result<ExtensionProcessOutput, String> {
        crate::platform::process::audit_command(
            request.audit_category,
            &request.executable,
            &request.args,
        );
        let mut process = ProcessRequest::new(&request.executable)
            .args(request.args)
            .timeout(request.timeout);
        if let Some(current_dir) = request.current_dir {
            process = process.current_dir(current_dir);
        }
        ProcessAdapter
            .execute(&process)
            .map(|output| ExtensionProcessOutput {
                success: output.success(),
                status: output.status_label(),
                stdout: output.stdout,
                stderr: output.stderr,
            })
            .map_err(process_error)
    }
}

fn process_error(error: ProcessError) -> String {
    match error {
        ProcessError::TimedOut {
            timeout_seconds,
            stderr,
            ..
        } => format!("Command timed out after {timeout_seconds} seconds. {stderr}"),
        other => other.to_string(),
    }
}

pub(super) fn platform_process_runner() -> Arc<dyn ExtensionProcessRunner> {
    Arc::new(PlatformExtensionProcessRunner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_keeps_executable_and_arguments_separate() {
        let request = ExtensionProcessRequest {
            executable: "python".to_string(),
            args: vec!["-c".to_string(), "literal; echo should-not-run".to_string()],
            current_dir: None,
            timeout: Duration::from_secs(1),
            audit_category: "extension.test",
        };
        let process = ProcessRequest::new(&request.executable).args(request.args.clone());
        let command = process.command().expect("command");

        assert_eq!(command.get_program(), "python");
        assert_eq!(
            command
                .get_args()
                .map(|value| value.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
            request.args
        );
    }
}
