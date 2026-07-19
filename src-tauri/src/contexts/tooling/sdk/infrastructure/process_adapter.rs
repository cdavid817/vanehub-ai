use crate::platform::process::{ProcessAdapter, ProcessError, ProcessRequest};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SdkProcessRequest {
    pub(super) executable: String,
    pub(super) args: Vec<String>,
    pub(super) current_dir: Option<PathBuf>,
    pub(super) timeout: Duration,
    pub(super) audit_category: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SdkProcessOutput {
    pub(super) success: bool,
    pub(super) stdout: String,
    pub(super) stderr: String,
    pub(super) status: String,
}

pub(super) trait SdkProcessRunner: Send + Sync {
    fn execute(&self, request: SdkProcessRequest) -> Result<SdkProcessOutput, String>;
}

#[derive(Debug, Clone, Copy, Default)]
struct PlatformSdkProcessRunner;

impl SdkProcessRunner for PlatformSdkProcessRunner {
    fn execute(&self, request: SdkProcessRequest) -> Result<SdkProcessOutput, String> {
        if let Some(category) = request.audit_category {
            crate::platform::process::audit_command(category, &request.executable, &request.args);
        }
        let mut process_request = ProcessRequest::new(&request.executable)
            .args(request.args)
            .timeout(request.timeout);
        if let Some(current_dir) = request.current_dir {
            process_request = process_request.current_dir(current_dir);
        }
        ProcessAdapter
            .execute(&process_request)
            .map(|output| {
                let success = output.success();
                let status = output.status_label();
                SdkProcessOutput {
                    success,
                    stdout: output.stdout,
                    stderr: output.stderr,
                    status,
                }
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

pub(super) fn platform_process_runner() -> Arc<dyn SdkProcessRunner> {
    Arc::new(PlatformSdkProcessRunner)
}
