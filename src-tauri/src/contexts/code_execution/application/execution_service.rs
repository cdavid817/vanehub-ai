use super::execution_support::{
    bounded_text, elapsed_ms, failed_result, limited_result, map_backend_launch,
    minimal_environment, output_media_type, safe_output_name,
};
use super::{
    CodeExecutionRequest, CodeExecutionResult, CodeExecutionStatus, CodeOutputArtifact,
    CodeRuntime, CodeSandboxReadiness, RuntimeVersion, SandboxLaunchRequest, SandboxOutputError,
    SandboxOutputFile, SandboxProcessBackend, SandboxProcessObservation, SandboxWorkspace,
    SandboxWorkspaceService, CODE_EXECUTION_CONTRACT_VERSION,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::execution_ports::{CodeOutputArtifactPort, CodeRuntimePort};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodeServiceError {
    InvalidRequest,
    IsolationUnavailable,
    RuntimeUnavailable,
    WorkspaceFailure,
    #[cfg(any(windows, test))]
    SpawnFailure,
    WaitFailure,
    OutputRejected,
    ArtifactFailure,
}

pub(crate) struct CodeExecutionService {
    workspaces: Arc<SandboxWorkspaceService>,
    backend: Arc<dyn SandboxProcessBackend>,
    runtimes: Arc<dyn CodeRuntimePort>,
    artifacts: Arc<dyn CodeOutputArtifactPort>,
}

impl CodeExecutionService {
    pub(crate) fn new(
        workspaces: Arc<SandboxWorkspaceService>,
        backend: Arc<dyn SandboxProcessBackend>,
        runtimes: Arc<dyn CodeRuntimePort>,
        artifacts: Arc<dyn CodeOutputArtifactPort>,
    ) -> Self {
        Self {
            workspaces,
            backend,
            runtimes,
            artifacts,
        }
    }

    pub(crate) fn execute(
        &self,
        request: CodeExecutionRequest,
        cancelled: Arc<AtomicBool>,
    ) -> Result<CodeExecutionResult, CodeServiceError> {
        let limits = request
            .validate()
            .map_err(|_| CodeServiceError::InvalidRequest)?;
        if !CodeSandboxReadiness::probe(self.backend.as_ref()).ready {
            return Err(CodeServiceError::IsolationUnavailable);
        }
        let (executable, _version) = self.runtimes.resolve_reviewed(request.runtime)?;
        let workspace = self
            .workspaces
            .create(&request)
            .map_err(|_| CodeServiceError::WorkspaceFailure)?;
        let started_at = Instant::now();
        let source_name = workspace
            .source_path
            .file_name()
            .ok_or(CodeServiceError::WorkspaceFailure)?
            .to_string_lossy()
            .into_owned();
        let mut arguments = Vec::with_capacity(request.arguments.len() + 2);
        if request.runtime == CodeRuntime::JavaScript {
            arguments.push("--preserve-symlinks-main".to_owned());
        }
        arguments.push(source_name);
        arguments.extend(request.arguments.clone());
        let launch = SandboxLaunchRequest {
            executable,
            arguments,
            working_directory: workspace.work_dir.clone(),
            environment: minimal_environment(request.runtime),
            limits,
        };
        let mut process = self.backend.launch(launch).map_err(map_backend_launch)?;
        let deadline = started_at + Duration::from_millis(limits.wall_time_ms);
        let terminal = loop {
            if cancelled.load(Ordering::Acquire) {
                process
                    .terminate_tree(Duration::from_secs(2))
                    .map_err(|_| CodeServiceError::WaitFailure)?;
                break Terminal::Cancelled;
            }
            let poll_deadline = deadline.min(Instant::now() + Duration::from_millis(50));
            if let Some(observation) = process
                .wait_until(poll_deadline)
                .map_err(|_| CodeServiceError::WaitFailure)?
            {
                break Terminal::Exited(observation);
            }
            match workspace.scan_outputs(limits) {
                Ok(_) => {}
                Err(SandboxOutputError::FileCountLimit) => {
                    process
                        .terminate_tree(Duration::from_secs(2))
                        .map_err(|_| CodeServiceError::WaitFailure)?;
                    break Terminal::LimitExceeded("file_count");
                }
                Err(SandboxOutputError::ByteLimit) => {
                    process
                        .terminate_tree(Duration::from_secs(2))
                        .map_err(|_| CodeServiceError::WaitFailure)?;
                    break Terminal::LimitExceeded("filesystem_bytes");
                }
                Err(SandboxOutputError::UnsafeFilesystem) => {
                    process
                        .terminate_tree(Duration::from_secs(2))
                        .map_err(|_| CodeServiceError::WaitFailure)?;
                    break Terminal::SandboxViolation;
                }
            }
            if Instant::now() >= deadline {
                process
                    .terminate_tree(Duration::from_secs(2))
                    .map_err(|_| CodeServiceError::WaitFailure)?;
                break Terminal::TimedOut;
            }
        };
        let result = self.finish(&request, &workspace, started_at, limits, terminal);
        let cleanup = workspace.cleanup();
        match (result, cleanup) {
            (_, Err(_)) => Ok(failed_result(
                &request.execution_id,
                started_at,
                CodeExecutionStatus::CleanupFailed,
                "Sandbox cleanup failed.",
            )),
            (result, Ok(())) => result,
        }
    }

    fn finish(
        &self,
        request: &CodeExecutionRequest,
        workspace: &SandboxWorkspace,
        started_at: Instant,
        limits: super::CodeExecutionLimits,
        terminal: Terminal,
    ) -> Result<CodeExecutionResult, CodeServiceError> {
        match terminal {
            Terminal::Cancelled => Ok(failed_result(
                &request.execution_id,
                started_at,
                CodeExecutionStatus::Cancelled,
                "Code execution was cancelled.",
            )),
            Terminal::TimedOut => Ok(failed_result(
                &request.execution_id,
                started_at,
                CodeExecutionStatus::TimedOut,
                "Code execution reached its time limit.",
            )),
            Terminal::LimitExceeded(reason) => Ok(limited_result(
                &request.execution_id,
                started_at,
                CodeExecutionStatus::LimitExceeded,
                reason,
                "Code execution exceeded its output limits.",
            )),
            Terminal::SandboxViolation => Ok(limited_result(
                &request.execution_id,
                started_at,
                CodeExecutionStatus::SandboxViolation,
                "unsafe_output",
                "Code execution produced an unsafe filesystem entry.",
            )),
            Terminal::Exited(observation) => match workspace.scan_outputs(limits) {
                Ok(outputs) => {
                    self.result_from_observation(request, outputs, started_at, limits, observation)
                }
                Err(SandboxOutputError::FileCountLimit) => Ok(limited_result(
                    &request.execution_id,
                    started_at,
                    CodeExecutionStatus::LimitExceeded,
                    "file_count",
                    "Code execution exceeded its output limits.",
                )),
                Err(SandboxOutputError::ByteLimit) => Ok(limited_result(
                    &request.execution_id,
                    started_at,
                    CodeExecutionStatus::LimitExceeded,
                    "filesystem_bytes",
                    "Code execution exceeded its output limits.",
                )),
                Err(SandboxOutputError::UnsafeFilesystem) => Ok(limited_result(
                    &request.execution_id,
                    started_at,
                    CodeExecutionStatus::SandboxViolation,
                    "unsafe_output",
                    "Code execution produced an unsafe filesystem entry.",
                )),
            },
        }
    }

    fn result_from_observation(
        &self,
        request: &CodeExecutionRequest,
        output_files: Vec<SandboxOutputFile>,
        started_at: Instant,
        limits: super::CodeExecutionLimits,
        observation: SandboxProcessObservation,
    ) -> Result<CodeExecutionResult, CodeServiceError> {
        let (stdout, stdout_truncated) = bounded_text(observation.stdout, limits.stdout_bytes);
        let (stderr, stderr_truncated) = bounded_text(observation.stderr, limits.stderr_bytes);
        let exceeded = stdout_truncated || stderr_truncated;
        let outputs = if observation.exit_code == 0 && !exceeded {
            self.admit_outputs(request, output_files, limits)?
        } else {
            Vec::new()
        };
        let status = if exceeded {
            CodeExecutionStatus::LimitExceeded
        } else if observation.exit_code == 0 {
            CodeExecutionStatus::Succeeded
        } else {
            CodeExecutionStatus::Failed
        };
        Ok(CodeExecutionResult {
            contract_version: CODE_EXECUTION_CONTRACT_VERSION,
            execution_id: request.execution_id.clone(),
            status,
            exit_code: Some(observation.exit_code),
            stdout,
            stderr,
            stdout_truncated,
            stderr_truncated,
            duration_ms: elapsed_ms(started_at),
            limit_reason: exceeded.then(|| "stdio_bytes".to_owned()),
            outputs,
            safe_error: (status != CodeExecutionStatus::Succeeded)
                .then(|| "Code execution did not complete successfully.".to_owned()),
        })
    }

    fn admit_outputs(
        &self,
        request: &CodeExecutionRequest,
        entries: Vec<SandboxOutputFile>,
        limits: super::CodeExecutionLimits,
    ) -> Result<Vec<CodeOutputArtifact>, CodeServiceError> {
        if entries.len() > limits.file_count as usize {
            return Err(CodeServiceError::OutputRejected);
        }
        let mut total = 0_u64;
        let mut outputs = Vec::with_capacity(entries.len());
        for entry in entries {
            total = total
                .checked_add(entry.bytes.len() as u64)
                .ok_or(CodeServiceError::OutputRejected)?;
            if total > limits.filesystem_bytes {
                return Err(CodeServiceError::OutputRejected);
            }
            let name = entry.name;
            if !safe_output_name(&name) {
                return Err(CodeServiceError::OutputRejected);
            }
            let bytes = entry.bytes;
            let media_type = output_media_type(&name, &bytes)?;
            outputs.push(self.artifacts.seal_output(
                &request.execution_id,
                &name,
                media_type,
                &bytes,
            )?);
        }
        Ok(outputs)
    }
}

enum Terminal {
    Exited(SandboxProcessObservation),
    Cancelled,
    TimedOut,
    LimitExceeded(&'static str),
    SandboxViolation,
}

#[cfg(test)]
#[path = "execution_service_tests.rs"]
mod tests;
