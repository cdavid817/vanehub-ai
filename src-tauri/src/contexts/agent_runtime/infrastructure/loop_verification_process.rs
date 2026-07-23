use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, LoopVerificationProcessPort, LoopVerificationProcessRequest,
    LoopVerificationProcessResult, LoopVerificationProcessStatus,
};
use crate::platform::process::{ProcessAdapter, ProcessCancellation, ProcessError, ProcessRequest};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const STREAM_OUTPUT_LIMIT: usize = 32 * 1024;
const MAX_ARGUMENTS: usize = 128;
const MAX_ARGUMENT_LENGTH: usize = 4096;
const MAX_TOTAL_ARGUMENT_LENGTH: usize = 32 * 1024;
const MAX_TIMEOUT_SECONDS: u64 = 60 * 60;

pub(crate) struct StructuredLoopVerificationProcess {
    allowed_programs: BTreeSet<String>,
}

impl Default for StructuredLoopVerificationProcess {
    fn default() -> Self {
        Self::new([
            "cargo",
            "dotnet",
            "go",
            "gradle",
            "gradlew",
            "gradlew.bat",
            "mvn",
            "node",
            "npm",
            "npm.cmd",
            "npx",
            "npx.cmd",
            "pytest",
            "python",
            "python3",
        ])
    }
}

impl StructuredLoopVerificationProcess {
    pub(crate) fn new(programs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            allowed_programs: programs
                .into_iter()
                .map(|program| program.into().to_ascii_lowercase())
                .collect(),
        }
    }

    fn prepare(
        &self,
        request: &LoopVerificationProcessRequest,
    ) -> Result<(PathBuf, String), AgentRuntimeApplicationError> {
        let root = canonical_directory(Path::new(&request.worktree_root), "worktree root")?;
        let working_directory = match request.command.working_directory.as_deref() {
            Some(relative) => {
                validate_relative_directory(relative)?;
                canonical_directory(&root.join(relative), "verification working directory")?
            }
            None => root.clone(),
        };
        if !working_directory.starts_with(&root) {
            return Err(policy_error(
                "verification working directory resolves outside the worktree root",
            ));
        }
        let program = request.command.program.trim();
        if program.is_empty()
            || program.contains(['/', '\\'])
            || program.as_bytes().get(1) == Some(&b':')
            || !self
                .allowed_programs
                .contains(&program.to_ascii_lowercase())
        {
            return Err(policy_error("verification executable is not allowed"));
        }
        validate_arguments(&request.command.args)?;
        if request.command.timeout_seconds == 0
            || request.command.timeout_seconds > MAX_TIMEOUT_SECONDS
        {
            return Err(policy_error(
                "verification timeout is outside the allowed range",
            ));
        }
        Ok((working_directory, program.to_string()))
    }
}

impl LoopVerificationProcessPort for StructuredLoopVerificationProcess {
    fn execute(
        &self,
        request: LoopVerificationProcessRequest,
    ) -> Result<LoopVerificationProcessResult, AgentRuntimeApplicationError> {
        let (working_directory, program) = self.prepare(&request)?;
        if request.cancellation.is_cancelled() {
            return Ok(empty_result(LoopVerificationProcessStatus::Cancelled));
        }

        let started_at = Instant::now();
        let process_request = ProcessRequest::new(program)
            .args(request.command.args.clone())
            .current_dir(working_directory)
            .timeout(Duration::from_secs(request.command.timeout_seconds))
            .cancellation(ProcessCancellation::from_signal(
                request.cancellation.signal(),
            ))
            .output_limit(STREAM_OUTPUT_LIMIT);

        let outcome = ProcessAdapter.execute(&process_request);
        let duration_ms = u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        match outcome {
            Ok(output) => Ok(LoopVerificationProcessResult {
                status: if output.success() {
                    LoopVerificationProcessStatus::Passed
                } else {
                    LoopVerificationProcessStatus::Failed
                },
                exit_code: output.status.code(),
                duration_ms,
                stdout: output.stdout,
                stderr: output.stderr,
                output_truncated: output.output_truncated,
            }),
            Err(ProcessError::TimedOut {
                stdout,
                stderr,
                output_truncated,
                ..
            }) => Ok(terminal_result(
                LoopVerificationProcessStatus::TimedOut,
                duration_ms,
                stdout,
                stderr,
                output_truncated,
            )),
            Err(ProcessError::Cancelled {
                stdout,
                stderr,
                output_truncated,
            }) => Ok(terminal_result(
                LoopVerificationProcessStatus::Cancelled,
                duration_ms,
                stdout,
                stderr,
                output_truncated,
            )),
            Err(error) => Err(process_error(error)),
        }
    }
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf, AgentRuntimeApplicationError> {
    let canonical = path
        .canonicalize()
        .map_err(|_| policy_error(format!("{label} is unavailable")))?;
    if !canonical.is_dir() {
        return Err(policy_error(format!("{label} is not a directory")));
    }
    Ok(canonical)
}

fn validate_relative_directory(value: &str) -> Result<(), AgentRuntimeApplicationError> {
    let path = Path::new(value);
    if value.trim().is_empty()
        || path.is_absolute()
        || value.as_bytes().get(1) == Some(&b':')
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(policy_error(
            "verification working directory must be a confined relative path",
        ));
    }
    Ok(())
}

fn validate_arguments(args: &[String]) -> Result<(), AgentRuntimeApplicationError> {
    let total = args.iter().map(String::len).sum::<usize>();
    if args.len() > MAX_ARGUMENTS
        || total > MAX_TOTAL_ARGUMENT_LENGTH
        || args.iter().any(|arg| {
            arg.len() > MAX_ARGUMENT_LENGTH
                || arg
                    .chars()
                    .any(|character| character == '\0' || character.is_control())
        })
    {
        return Err(policy_error(
            "verification arguments are invalid or too large",
        ));
    }
    Ok(())
}

fn empty_result(status: LoopVerificationProcessStatus) -> LoopVerificationProcessResult {
    LoopVerificationProcessResult {
        status,
        exit_code: None,
        duration_ms: 0,
        stdout: String::new(),
        stderr: String::new(),
        output_truncated: false,
    }
}

fn terminal_result(
    status: LoopVerificationProcessStatus,
    duration_ms: u64,
    stdout: String,
    stderr: String,
    output_truncated: bool,
) -> LoopVerificationProcessResult {
    LoopVerificationProcessResult {
        status,
        exit_code: None,
        duration_ms,
        stdout,
        stderr,
        output_truncated,
    }
}

fn policy_error(message: impl Into<String>) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::VerificationPolicy(message.into())
}

fn process_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::VerificationProcess(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::application::{
        LoopVerificationCancellation, LoopVerificationCommandView,
    };
    use std::thread;

    fn fixture_root() -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("vanehub-loop-verify-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("nested")).expect("fixture root");
        root
    }

    fn request(root: &Path, program: &str) -> LoopVerificationProcessRequest {
        LoopVerificationProcessRequest {
            worktree_root: root.to_string_lossy().to_string(),
            command: LoopVerificationCommandView {
                id: "check-1".to_string(),
                program: program.to_string(),
                args: Vec::new(),
                working_directory: None,
                timeout_seconds: 30,
                required: true,
            },
            cancellation: LoopVerificationCancellation::default(),
        }
    }

    #[test]
    fn policy_rejects_disallowed_program_traversal_and_control_arguments() {
        let root = fixture_root();
        let process = StructuredLoopVerificationProcess::new(["cargo"]);

        assert!(matches!(
            process.execute(request(&root, "powershell")),
            Err(AgentRuntimeApplicationError::VerificationPolicy(_))
        ));
        let mut traversal = request(&root, "cargo");
        traversal.command.working_directory = Some("../outside".to_string());
        assert!(matches!(
            process.execute(traversal),
            Err(AgentRuntimeApplicationError::VerificationPolicy(_))
        ));
        let mut control = request(&root, "cargo");
        control.command.args = vec!["test\nnext".to_string()];
        assert!(matches!(
            process.execute(control),
            Err(AgentRuntimeApplicationError::VerificationPolicy(_))
        ));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn pre_cancelled_request_never_launches_and_output_is_bounded() {
        let root = fixture_root();
        let process = StructuredLoopVerificationProcess::new(["program-not-installed"]);
        let cancelled = LoopVerificationCancellation::default();
        cancelled.cancel();
        let mut request = request(&root, "program-not-installed");
        request.cancellation = cancelled;

        let result = process.execute(request).expect("cancelled result");
        assert_eq!(result.status, LoopVerificationProcessStatus::Cancelled);

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn running_process_honors_timeout_and_cancellation() {
        let root = fixture_root();
        let (program, args) = sleep_command();

        let process = StructuredLoopVerificationProcess::new([program]);
        let mut timed = request(&root, program);
        timed.command.args = args.clone();
        timed.command.timeout_seconds = 1;
        let result = process.execute(timed).expect("timed result");
        assert_eq!(result.status, LoopVerificationProcessStatus::TimedOut);

        let process = StructuredLoopVerificationProcess::new([program]);
        let cancellation = LoopVerificationCancellation::default();
        let mut cancelled = request(&root, program);
        cancelled.command.args = args;
        cancelled.command.timeout_seconds = 10;
        cancelled.cancellation = cancellation.clone();
        let running = thread::spawn(move || process.execute(cancelled));
        thread::sleep(Duration::from_millis(100));
        cancellation.cancel();
        let result = running
            .join()
            .expect("verification thread")
            .expect("cancelled result");
        assert_eq!(result.status, LoopVerificationProcessStatus::Cancelled);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[cfg(target_os = "windows")]
    fn sleep_command() -> (&'static str, Vec<String>) {
        (
            "powershell",
            vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                "Start-Sleep -Seconds 5".to_string(),
            ],
        )
    }

    #[cfg(not(target_os = "windows"))]
    fn sleep_command() -> (&'static str, Vec<String>) {
        ("sh", vec!["-c".to_string(), "sleep 5".to_string()])
    }
}
