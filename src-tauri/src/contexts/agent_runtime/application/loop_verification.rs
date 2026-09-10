use super::{
    AgentClockPort, AgentLogLevel, AgentRuntimeApplicationError, LoopEvidenceView,
    LoopIterationRepository, LoopOperationContext, LoopOperationKind, LoopOperationObserver,
    LoopScopePlatformPort, LoopVerificationBatchResult, LoopVerificationCommandView,
    LoopVerificationProcessPort, LoopVerificationProcessRequest, LoopVerificationProcessStatus,
    LoopVerificationScope, RunLoopVerificationRequest, LOOP_SCOPE_RUNTIME_REVISION,
};
use crate::contexts::agent_runtime::domain::{
    LoopCoverage, LoopRequestedMode, LoopVerificationKind,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct LoopVerificationApplicationPorts {
    pub(crate) iterations: Arc<dyn LoopIterationRepository>,
    pub(crate) processes: Arc<dyn LoopVerificationProcessPort>,
    pub(crate) observer: LoopOperationObserver,
    pub(crate) clock: Arc<dyn AgentClockPort>,
    pub(crate) evidence: Arc<dyn LoopVerificationEvidencePort>,
    pub(crate) scope_platform: Arc<dyn LoopScopePlatformPort>,
}

pub(crate) trait LoopVerificationEvidencePort: Send + Sync {
    fn project(&self, fact: LoopVerificationEvidenceFact);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopVerificationEvidenceFact {
    pub(crate) run_id: String,
    pub(crate) iteration_id: String,
    pub(crate) workspace: String,
    pub(crate) occurred_at: String,
    pub(crate) passed_count: u32,
    pub(crate) failed_count: u32,
    pub(crate) cancelled: bool,
}

#[derive(Clone)]
pub(crate) struct LoopVerificationApplicationService {
    ports: LoopVerificationApplicationPorts,
}

/// One verification outcome as the evidence row records it.
struct CommandOutcome {
    status: &'static str,
    passed: bool,
    cancelled: bool,
    summary: String,
    exit_code: Option<i32>,
    duration_ms: Option<u64>,
    output_truncated: bool,
    extra: serde_json::Value,
}

impl LoopVerificationApplicationService {
    pub(crate) fn new(ports: LoopVerificationApplicationPorts) -> Self {
        Self { ports }
    }

    pub(crate) fn run_commands(
        &self,
        request: RunLoopVerificationRequest,
    ) -> Result<LoopVerificationBatchResult, AgentRuntimeApplicationError> {
        validate_request(&request)?;
        let scope = request.scope.clone().ok_or_else(|| {
            AgentRuntimeApplicationError::Validation(
                "Loop verification requires the frozen run scope.".to_string(),
            )
        })?;
        let mode = scope
            .binding
            .mode()
            .unwrap_or(LoopRequestedMode::PreventiveRequired);
        let mut evidence = Vec::with_capacity(request.commands.len());
        let mut required_checks_passed = true;
        let mut cancelled = false;

        for command in request.commands {
            let context = LoopOperationContext {
                run_id: request.run_id.clone(),
                iteration_id: Some(request.iteration_id.clone()),
                kind: LoopOperationKind::Verification,
            };
            let operation = self.ports.observer.start(
                context,
                &format!("Running verification command {}", command.id),
            )?;
            let fingerprint = verification_fingerprint(&command, &scope);
            let outcome = match command.kind {
                LoopVerificationKind::NativeCheck => {
                    self.run_native_check(&request.run_id, &request.worktree_root, &scope, &command)
                }
                LoopVerificationKind::Process => {
                    // Strict mode never admits an uncontained process; refusing here protects a
                    // run whose definition was somehow queued without that check.
                    if mode == LoopRequestedMode::PreventiveRequired
                        && !LoopCoverage::ArtifactValidationOnly.satisfies(mode)
                    {
                        Err(AgentRuntimeApplicationError::VerificationPolicy(
                            "process verification is not admitted under preventive-required"
                                .to_string(),
                        ))
                    } else {
                        self.run_process(
                            &request.worktree_root,
                            &command,
                            &request.cancellation,
                            &operation,
                        )
                    }
                }
            };

            let item = match outcome {
                Ok(outcome) => {
                    if command.required && !outcome.passed {
                        required_checks_passed = false;
                    }
                    if outcome.cancelled {
                        cancelled = true;
                    }
                    let evidence = self.evidence(
                        &request.run_id,
                        &request.iteration_id,
                        &operation.id,
                        &command,
                        outcome.status,
                        outcome.summary.clone(),
                        outcome.exit_code,
                        outcome.duration_ms,
                        outcome.output_truncated,
                        &fingerprint,
                        outcome.extra,
                    );
                    self.ports.iterations.append_evidence(&evidence)?;
                    match outcome.status {
                        "passed" => self
                            .ports
                            .observer
                            .complete(&operation, &evidence.summary)?,
                        "cancelled" => self.ports.observer.cancel(&operation, &evidence.summary)?,
                        _ => self.ports.observer.fail(&operation, &evidence.summary)?,
                    }
                    evidence
                }
                Err(error) => {
                    if command.required {
                        required_checks_passed = false;
                    }
                    let summary = format!("Verification command {} could not run.", command.id);
                    let evidence = self.evidence(
                        &request.run_id,
                        &request.iteration_id,
                        &operation.id,
                        &command,
                        "error",
                        summary,
                        None,
                        None,
                        false,
                        &fingerprint,
                        json!({ "error": error.to_string() }),
                    );
                    self.ports.iterations.append_evidence(&evidence)?;
                    self.ports.observer.fail(&operation, &error.to_string())?;
                    evidence
                }
            };
            evidence.push(item);
            if cancelled {
                break;
            }
        }

        let passed_count = evidence
            .iter()
            .filter(|item| item.status == "passed")
            .count() as u32;
        let failed_count = evidence.len() as u32 - passed_count;
        self.ports.evidence.project(LoopVerificationEvidenceFact {
            run_id: request.run_id.clone(),
            iteration_id: request.iteration_id.clone(),
            workspace: request.worktree_root,
            occurred_at: self.ports.clock.now(),
            passed_count,
            failed_count,
            cancelled,
        });
        Ok(LoopVerificationBatchResult {
            evidence,
            required_checks_passed: required_checks_passed && !cancelled,
            cancelled,
        })
    }

    fn run_native_check(
        &self,
        run_id: &str,
        worktree_root: &str,
        scope: &LoopVerificationScope,
        command: &LoopVerificationCommandView,
    ) -> Result<CommandOutcome, AgentRuntimeApplicationError> {
        let started = std::time::Instant::now();
        let view = self.ports.scope_platform.native_check(
            run_id,
            worktree_root,
            &scope.binding.root,
            &scope.binding.baseline_manifest_id,
            &scope.input_manifest_id,
        )?;
        let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let status: &'static str = match view.status.as_str() {
            "passed" => "passed",
            "failed" => "failed",
            _ => "unverifiable",
        };
        Ok(CommandOutcome {
            status,
            passed: status == "passed",
            cancelled: false,
            summary: match status {
                "passed" => format!(
                    "Native check {} passed ({} text file(s) inspected, {} binary excluded).",
                    command.id, view.inspected_files, view.binary_excluded
                ),
                "failed" => format!(
                    "Native check {} failed with {} finding(s).",
                    command.id,
                    view.findings.len()
                ),
                _ => format!(
                    "Native check {} is unverifiable: {}",
                    command.id,
                    view.detail.clone().unwrap_or_default()
                ),
            },
            exit_code: None,
            duration_ms: Some(duration_ms),
            output_truncated: false,
            extra: json!({
                "kind": "native-check",
                "findings": view.findings,
                "inspectedFiles": view.inspected_files,
                "binaryExcluded": view.binary_excluded,
                "detail": view.detail,
            }),
        })
    }

    fn run_process(
        &self,
        worktree_root: &str,
        command: &LoopVerificationCommandView,
        cancellation: &super::LoopVerificationCancellation,
        operation: &super::ActiveLoopOperation,
    ) -> Result<CommandOutcome, AgentRuntimeApplicationError> {
        let result = self
            .ports
            .processes
            .execute(LoopVerificationProcessRequest {
                worktree_root: worktree_root.to_string(),
                command: command.clone(),
                cancellation: cancellation.clone(),
            })?;
        self.record_output(operation, &result.stdout, &result.stderr)?;
        let status = status_name(result.status);
        Ok(CommandOutcome {
            status,
            passed: result.status == LoopVerificationProcessStatus::Passed,
            cancelled: result.status == LoopVerificationProcessStatus::Cancelled,
            summary: summary(&command.id, result.status),
            exit_code: result.exit_code,
            duration_ms: Some(result.duration_ms),
            output_truncated: result.output_truncated,
            extra: json!({ "kind": "process" }),
        })
    }

    fn record_output(
        &self,
        operation: &super::ActiveLoopOperation,
        stdout: &str,
        stderr: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        if !stdout.is_empty() {
            self.ports.observer.record(
                &operation.context,
                Some(&operation.id),
                AgentLogLevel::Info,
                &format!("stdout:\n{stdout}"),
            )?;
        }
        if !stderr.is_empty() {
            self.ports.observer.record(
                &operation.context,
                Some(&operation.id),
                AgentLogLevel::Warn,
                &format!("stderr:\n{stderr}"),
            )?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn evidence(
        &self,
        run_id: &str,
        iteration_id: &str,
        operation_id: &str,
        command: &LoopVerificationCommandView,
        status: &str,
        summary: String,
        exit_code: Option<i32>,
        duration_ms: Option<u64>,
        output_truncated: bool,
        fingerprint: &str,
        extra: serde_json::Value,
    ) -> LoopEvidenceView {
        let mut details = json!({
            "required": command.required,
            "outputTruncated": output_truncated,
            "fingerprint": fingerprint,
            "verificationKind": command.kind.as_str(),
        });
        if let (Some(target), Some(source)) = (details.as_object_mut(), extra.as_object()) {
            for (key, value) in source {
                target.insert(key.clone(), value.clone());
            }
        }
        LoopEvidenceView {
            id: format!("loop-evidence-{}", Uuid::new_v4()),
            run_id: run_id.to_string(),
            iteration_id: Some(iteration_id.to_string()),
            kind: "verification-command".to_string(),
            status: status.to_string(),
            summary,
            operation_id: Some(operation_id.to_string()),
            command_id: Some(command.id.clone()),
            exit_code,
            duration_ms,
            details: Some(details),
            created_at: self.ports.clock.now(),
        }
    }
}

/// The reuse key for verification evidence. Only an identical command over an identical input
/// snapshot, scope, assessment and runtime revision may be treated as current.
pub(crate) fn verification_fingerprint(
    command: &LoopVerificationCommandView,
    scope: &LoopVerificationScope,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(LOOP_SCOPE_RUNTIME_REVISION.as_bytes());
    hasher.update(b"\0");
    hasher.update(command.id.as_bytes());
    hasher.update(b"\0");
    hasher.update(command.kind.as_str().as_bytes());
    hasher.update(b"\0");
    hasher.update(command.program.as_bytes());
    hasher.update(b"\0");
    for arg in &command.args {
        hasher.update(arg.as_bytes());
        hasher.update(b"\x1f");
    }
    hasher.update(b"\0");
    hasher.update(
        command
            .working_directory
            .as_deref()
            .unwrap_or(".")
            .as_bytes(),
    );
    hasher.update(b"\0");
    hasher.update(command.timeout_seconds.to_le_bytes());
    hasher.update(if command.required {
        b"\0required\0"
    } else {
        b"\0optional\0"
    });
    hasher.update(scope.input_digest.as_bytes());
    hasher.update(b"\0");
    hasher.update(scope.binding.scope_digest.as_bytes());
    hasher.update(b"\0");
    hasher.update(scope.binding.witness_digest.as_bytes());
    hasher.update(b"\0");
    hasher.update(scope.assessment_digest.as_bytes());
    format!(
        "sha256:{}",
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

fn validate_request(
    request: &RunLoopVerificationRequest,
) -> Result<(), AgentRuntimeApplicationError> {
    if request.run_id.trim().is_empty()
        || request.iteration_id.trim().is_empty()
        || request.worktree_root.trim().is_empty()
        || request.commands.is_empty()
    {
        return Err(AgentRuntimeApplicationError::Validation(
            "Loop verification request is incomplete.".to_string(),
        ));
    }
    Ok(())
}

fn status_name(status: LoopVerificationProcessStatus) -> &'static str {
    match status {
        LoopVerificationProcessStatus::Passed => "passed",
        LoopVerificationProcessStatus::Failed => "failed",
        LoopVerificationProcessStatus::TimedOut => "timed-out",
        LoopVerificationProcessStatus::Cancelled => "cancelled",
    }
}

fn summary(command_id: &str, status: LoopVerificationProcessStatus) -> String {
    format!("Verification command {command_id} {}.", status_name(status))
}
