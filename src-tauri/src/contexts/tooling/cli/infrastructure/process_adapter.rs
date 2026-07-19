use crate::platform::process::{ProcessAdapter, ProcessError, ProcessRequest};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CliProcessRequest {
    pub(super) executable: String,
    pub(super) args: Vec<String>,
    pub(super) timeout: Duration,
    pub(super) audit_category: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CliProcessOutput {
    pub(super) success: bool,
    pub(super) stdout: String,
    pub(super) stderr: String,
    pub(super) status: String,
}

pub(super) trait CliProcessRunner: Send + Sync {
    fn execute(&self, request: CliProcessRequest) -> Result<CliProcessOutput, String>;
}

#[derive(Debug, Clone, Copy, Default)]
struct PlatformCliProcessRunner;

impl CliProcessRunner for PlatformCliProcessRunner {
    fn execute(&self, request: CliProcessRequest) -> Result<CliProcessOutput, String> {
        if let Some(category) = request.audit_category {
            crate::platform::process::audit_command(category, &request.executable, &request.args);
        }
        let process_request = ProcessRequest::new(&request.executable)
            .args(request.args)
            .timeout(request.timeout);
        ProcessAdapter
            .execute(&process_request)
            .map(|output| CliProcessOutput {
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
        ProcessError::InvalidExecutable(message) => message.to_string(),
        ProcessError::TimedOut { .. } => "command timed out".to_string(),
        other => other.to_string(),
    }
}

pub(super) fn platform_process_runner() -> Arc<dyn CliProcessRunner> {
    Arc::new(PlatformCliProcessRunner)
}
