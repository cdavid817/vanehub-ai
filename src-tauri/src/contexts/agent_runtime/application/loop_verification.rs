use super::{
    AgentClockPort, AgentLogLevel, AgentRuntimeApplicationError, LoopEvidenceView,
    LoopIterationRepository, LoopOperationContext, LoopOperationKind, LoopOperationObserver,
    LoopVerificationBatchResult, LoopVerificationProcessPort, LoopVerificationProcessRequest,
    LoopVerificationProcessStatus, RunLoopVerificationRequest,
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct LoopVerificationApplicationPorts {
    pub(crate) iterations: Arc<dyn LoopIterationRepository>,
    pub(crate) processes: Arc<dyn LoopVerificationProcessPort>,
    pub(crate) observer: LoopOperationObserver,
    pub(crate) clock: Arc<dyn AgentClockPort>,
}

#[derive(Clone)]
pub(crate) struct LoopVerificationApplicationService {
    ports: LoopVerificationApplicationPorts,
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
            let result = self
                .ports
                .processes
                .execute(LoopVerificationProcessRequest {
                    worktree_root: request.worktree_root.clone(),
                    command: command.clone(),
                    cancellation: request.cancellation.clone(),
                });

            let item = match result {
                Ok(result) => {
                    self.record_output(&operation, &result.stdout, &result.stderr)?;
                    let status = status_name(result.status);
                    let passed = result.status == LoopVerificationProcessStatus::Passed;
                    if command.required && !passed {
                        required_checks_passed = false;
                    }
                    if result.status == LoopVerificationProcessStatus::Cancelled {
                        cancelled = true;
                    }
                    let evidence = self.evidence(
                        &request.run_id,
                        &request.iteration_id,
                        &operation.id,
                        &command.id,
                        status,
                        summary(&command.id, result.status),
                        result.exit_code,
                        Some(result.duration_ms),
                        command.required,
                        result.output_truncated,
                    );
                    self.ports.iterations.append_evidence(&evidence)?;
                    self.finish_operation(&operation, result.status, &evidence.summary)?;
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
                        &command.id,
                        "error",
                        summary,
                        None,
                        None,
                        command.required,
                        false,
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

        Ok(LoopVerificationBatchResult {
            evidence,
            required_checks_passed: required_checks_passed && !cancelled,
            cancelled,
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

    fn finish_operation(
        &self,
        operation: &super::ActiveLoopOperation,
        status: LoopVerificationProcessStatus,
        summary: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        match status {
            LoopVerificationProcessStatus::Passed => {
                self.ports.observer.complete(operation, summary)
            }
            LoopVerificationProcessStatus::Cancelled => {
                self.ports.observer.cancel(operation, summary)
            }
            LoopVerificationProcessStatus::Failed | LoopVerificationProcessStatus::TimedOut => {
                self.ports.observer.fail(operation, summary)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn evidence(
        &self,
        run_id: &str,
        iteration_id: &str,
        operation_id: &str,
        command_id: &str,
        status: &str,
        summary: String,
        exit_code: Option<i32>,
        duration_ms: Option<u64>,
        required: bool,
        output_truncated: bool,
    ) -> LoopEvidenceView {
        LoopEvidenceView {
            id: format!("loop-evidence-{}", Uuid::new_v4()),
            run_id: run_id.to_string(),
            iteration_id: Some(iteration_id.to_string()),
            kind: "verification-command".to_string(),
            status: status.to_string(),
            summary,
            operation_id: Some(operation_id.to_string()),
            command_id: Some(command_id.to_string()),
            exit_code,
            duration_ms,
            details: Some(json!({
                "required": required,
                "outputTruncated": output_truncated,
            })),
            created_at: self.ports.clock.now(),
        }
    }
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
