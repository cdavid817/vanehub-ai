use crate::contexts::agent_runtime::application::{
    AgentChatConfiguration, AgentCliProfileGateway, AgentProcessEventSink, AgentProcessGateway,
    AgentRegistryRepository, AgentRuntimeApplicationError, AgentSession,
    CoordinationExecutionOutput, CoordinationExecutionRequest, CoordinationExecutionResult,
    CoordinationNodeExecutor, GenerationProcessEvent, GenerationProcessFailureKind,
    GenerationProcessRequest, ProcessStopInitiator,
};
use crate::contexts::agent_runtime::domain::{
    AgentAvailability, AgentLifecycle, CoordinationFailureKind, CoordinationRunStatus,
    InteractionMode, COORDINATION_OUTPUT_LIMIT_BYTES,
};
use crate::contexts::execution_observability::api::{
    CapturePolicy, ExecutionContext, ExecutionFidelity, ExecutionIdentityPort, ExecutionRun,
    ExecutionSource, ExecutionSpan, ExecutionStatus, ExecutionTelemetryPort, SafeAttributeValue,
    SafeAttributes,
};
use std::collections::{HashMap, HashSet};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

const COORDINATION_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(30 * 60);

#[derive(Clone)]
pub(crate) struct NativeCoordinationNodeExecutor {
    registry: Arc<dyn AgentRegistryRepository>,
    cli_profiles: Arc<dyn AgentCliProfileGateway>,
    processes: Arc<dyn AgentProcessGateway>,
    execution_ids: Arc<dyn ExecutionIdentityPort>,
    telemetry: Arc<dyn ExecutionTelemetryPort>,
    active: Arc<Mutex<HashMap<String, String>>>,
    cancelled: Arc<Mutex<HashSet<String>>>,
    execution_contexts: Arc<Mutex<HashMap<String, ExecutionContext>>>,
}

impl NativeCoordinationNodeExecutor {
    pub(crate) fn new(
        registry: Arc<dyn AgentRegistryRepository>,
        cli_profiles: Arc<dyn AgentCliProfileGateway>,
        processes: Arc<dyn AgentProcessGateway>,
        execution_ids: Arc<dyn ExecutionIdentityPort>,
        telemetry: Arc<dyn ExecutionTelemetryPort>,
    ) -> Self {
        Self {
            registry,
            cli_profiles,
            processes,
            execution_ids,
            telemetry,
            active: Arc::new(Mutex::new(HashMap::new())),
            cancelled: Arc::new(Mutex::new(HashSet::new())),
            execution_contexts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn cancelled(&self, run_id: &str) -> bool {
        self.cancelled
            .lock()
            .map(|cancelled| cancelled.contains(run_id))
            .unwrap_or(true)
    }

    fn start_attempt_telemetry(
        &self,
        request: &CoordinationExecutionRequest,
    ) -> Result<ExecutionContext, AgentRuntimeApplicationError> {
        let root_context = self.ensure_root_telemetry(&request.run_id, &request.operation_id)?;
        let attempt_context = ExecutionContext {
            run_id: root_context.run_id.clone(),
            trace_id: root_context.trace_id.clone(),
            span_id: self.execution_ids.next_span_id(),
            capture_policy: root_context.capture_policy,
            sampling_per_million: root_context.sampling_per_million,
            mcp_relay_enabled: root_context.mcp_relay_enabled,
        };
        let candidate_role = match request.candidate_role {
            crate::contexts::agent_runtime::domain::CoordinationCandidateRole::Primary => "primary",
            crate::contexts::agent_runtime::domain::CoordinationCandidateRole::Fallback => {
                "fallback"
            }
        };
        let _ = self.telemetry.start_span(&ExecutionSpan {
            context: attempt_context.clone(),
            parent_span_id: Some(root_context.span_id),
            name: "vanehub.coordination.agent_attempt".to_string(),
            status: ExecutionStatus::Running,
            fidelity: ExecutionFidelity::Native,
            started_at: chrono::Utc::now().to_rfc3339(),
            ended_at: None,
            error_classification: None,
            attributes: safe_attributes([
                (
                    "vanehub.coordination.node_id".to_string(),
                    SafeAttributeValue::String(request.node_id.clone()),
                ),
                (
                    "vanehub.coordination.candidate_role".to_string(),
                    SafeAttributeValue::String(candidate_role.to_string()),
                ),
                (
                    "vanehub.coordination.attempt".to_string(),
                    SafeAttributeValue::Integer(i64::from(request.attempt)),
                ),
                (
                    "vanehub.agent.id".to_string(),
                    SafeAttributeValue::String(request.agent_id.clone()),
                ),
            ]),
            links: Vec::new(),
        });
        let _ = self.telemetry.add_metric(
            "vanehub.coordination.agent_attempt.started",
            1,
            &[("candidate_role", candidate_role)],
        );
        Ok(attempt_context)
    }

    fn ensure_root_telemetry(
        &self,
        run_id: &str,
        operation_id: &str,
    ) -> Result<ExecutionContext, AgentRuntimeApplicationError> {
        let mut contexts = self.execution_contexts.lock().map_err(executor_error)?;
        if let Some(context) = contexts.get(run_id) {
            return Ok(context.clone());
        }
        let context = self
            .execution_ids
            .next_context(CapturePolicy::MetadataOnly, 1.0, false);
        let now = chrono::Utc::now().to_rfc3339();
        let attributes = safe_attributes([
            (
                "vanehub.stage".to_string(),
                SafeAttributeValue::String("multi_agent_coordination".to_string()),
            ),
            (
                "vanehub.coordination.run_id".to_string(),
                SafeAttributeValue::String(run_id.to_string()),
            ),
        ]);
        let _ = self.telemetry.start_run(&ExecutionRun {
            context: context.clone(),
            source: ExecutionSource::Desktop,
            status: ExecutionStatus::Running,
            started_at: now.clone(),
            ended_at: None,
            error_classification: None,
            session_id: None,
            user_message_id: None,
            assistant_message_id: None,
            operation_id: Some(operation_id.to_string()),
            agent_id: None,
            provider_session_id: None,
            attributes: attributes.clone(),
            links: Vec::new(),
        });
        let _ = self.telemetry.start_span(&ExecutionSpan {
            context: context.clone(),
            parent_span_id: None,
            name: "vanehub.coordination.execute".to_string(),
            status: ExecutionStatus::Running,
            fidelity: ExecutionFidelity::Native,
            started_at: now,
            ended_at: None,
            error_classification: None,
            attributes,
            links: Vec::new(),
        });
        contexts.insert(run_id.to_string(), context.clone());
        Ok(context)
    }

    fn finish_attempt_telemetry(
        &self,
        context: &ExecutionContext,
        status: ExecutionStatus,
        error: Option<&str>,
    ) {
        let ended_at = chrono::Utc::now().to_rfc3339();
        let _ =
            self.telemetry
                .finish_span(&context.run_id, &context.span_id, status, &ended_at, error);
        let outcome = match status {
            ExecutionStatus::Succeeded => "succeeded",
            ExecutionStatus::Cancelled => "cancelled",
            ExecutionStatus::Accepted
            | ExecutionStatus::Running
            | ExecutionStatus::Failed
            | ExecutionStatus::Incomplete => "failed",
        };
        let _ = self.telemetry.add_metric(
            "vanehub.coordination.agent_attempt.completed",
            1,
            &[("outcome", outcome)],
        );
    }
}

impl CoordinationNodeExecutor for NativeCoordinationNodeExecutor {
    fn start_coordination(
        &self,
        run_id: &str,
        operation_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.ensure_root_telemetry(run_id, operation_id).map(|_| ())
    }

    fn execute(
        &self,
        request: CoordinationExecutionRequest,
    ) -> Result<CoordinationExecutionResult, AgentRuntimeApplicationError> {
        if self.cancelled(&request.run_id) {
            return Ok(cancelled_result());
        }
        let execution_context = self.start_attempt_telemetry(&request)?;
        let Some(agent) = self.registry.find(&request.agent_id)? else {
            self.finish_attempt_telemetry(
                &execution_context,
                ExecutionStatus::Failed,
                Some("agent_not_found"),
            );
            return Ok(retryable(format!(
                "Agent not found at execution time: {}",
                request.agent_id
            )));
        };
        if agent.availability().state() != AgentAvailability::Available {
            self.finish_attempt_telemetry(
                &execution_context,
                ExecutionStatus::Failed,
                Some("agent_unavailable"),
            );
            return Ok(retryable(
                agent
                    .availability()
                    .reason()
                    .unwrap_or("Agent is unavailable at execution time.")
                    .to_string(),
            ));
        }
        if !agent.supports(InteractionMode::Cli) {
            self.finish_attempt_telemetry(
                &execution_context,
                ExecutionStatus::Failed,
                Some("interaction_mode_unsupported"),
            );
            return Ok(non_retryable(format!(
                "Agent {} does not support CLI coordination execution.",
                request.agent_id
            )));
        }
        let configuration = AgentChatConfiguration {
            agent_id: request.agent_id.clone(),
            interaction_mode: InteractionMode::Cli,
            permission_mode: "default".to_string(),
            provider_id: None,
            model_id: None,
            reasoning_depth: None,
            streaming: true,
            thinking: true,
            long_context: true,
        };
        let cli_profile = match self.cli_profiles.load(&request.agent_id, &configuration) {
            Ok(profile) => profile,
            Err(error) => {
                self.finish_attempt_telemetry(
                    &execution_context,
                    ExecutionStatus::Failed,
                    Some("cli_profile_unavailable"),
                );
                return Ok(classify_application_error(error));
            }
        };
        let session_id = format!("{}-{}-{}", request.run_id, request.node_id, request.attempt);
        let effective_prompt = if request.prerequisite_context.is_empty() {
            request.instruction.clone()
        } else {
            format!(
                "{}\n\nPrerequisite Agent outputs:\n{}",
                request.instruction, request.prerequisite_context
            )
        };
        let started = match self.processes.start_generation(GenerationProcessRequest {
            execution_context: execution_context.clone(),
            session: AgentSession {
                id: session_id.clone(),
                agent_id: request.agent_id.clone(),
                interaction_mode: InteractionMode::Cli,
                lifecycle: AgentLifecycle::Running,
                folder: request.project_path.clone(),
                runtime_session_id: None,
                archived: false,
                read_only: false,
                loop_ownership: None,
            },
            agent: (&agent).into(),
            message_id: format!("{session_id}-message"),
            operation_id: request.operation_id.clone(),
            configuration,
            effective_prompt,
            cli_profile,
        }) {
            Ok(started) => started,
            Err(error) => {
                self.finish_attempt_telemetry(
                    &execution_context,
                    ExecutionStatus::Failed,
                    Some("process_start_failed"),
                );
                return Ok(classify_application_error(error));
            }
        };
        self.active
            .lock()
            .map_err(executor_error)?
            .insert(request.run_id.clone(), started.process_id.clone());
        let (sender, receiver) = mpsc::channel();
        if let Err(error) = self.processes.monitor_generation(
            &started.process_id,
            Arc::new(CoordinationProcessSink {
                sender: Mutex::new(sender),
            }),
        ) {
            self.active
                .lock()
                .map_err(executor_error)?
                .remove(&request.run_id);
            let _ = self
                .processes
                .stop_generation(&started.process_id, ProcessStopInitiator::RuntimeCleanup);
            self.finish_attempt_telemetry(
                &execution_context,
                ExecutionStatus::Failed,
                Some("process_monitor_failed"),
            );
            return Ok(classify_application_error(error));
        }
        let mut output = BoundedOutputAccumulator::default();
        let result = loop {
            match receiver.recv_timeout(COORDINATION_ATTEMPT_TIMEOUT) {
                Ok(GenerationProcessEvent::Token(content)) => output.push(&content),
                Ok(GenerationProcessEvent::Completed) => {
                    break CoordinationExecutionResult::Succeeded(output.finish());
                }
                Ok(GenerationProcessEvent::Failed(failure)) => {
                    break if self.cancelled(&request.run_id) {
                        cancelled_result()
                    } else {
                        match failure.kind {
                            GenerationProcessFailureKind::Retryable => {
                                retryable(failure.diagnostic)
                            }
                            GenerationProcessFailureKind::NonRetryable => {
                                non_retryable(failure.diagnostic)
                            }
                        }
                    };
                }
                Ok(
                    GenerationProcessEvent::Thinking(_)
                    | GenerationProcessEvent::ToolUse(_)
                    | GenerationProcessEvent::ToolLifecycle(_)
                    | GenerationProcessEvent::RichBlock(_)
                    | GenerationProcessEvent::RuntimeSessionId(_)
                    | GenerationProcessEvent::Stderr(_),
                ) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let _ = self
                        .processes
                        .stop_generation(&started.process_id, ProcessStopInitiator::RuntimeCleanup);
                    break retryable("Coordination Agent attempt timed out.".to_string());
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break retryable(
                        "Coordination Agent process ended without a terminal event.".to_string(),
                    );
                }
            }
        };
        self.active
            .lock()
            .map_err(executor_error)?
            .remove(&request.run_id);
        match &result {
            CoordinationExecutionResult::Succeeded(_) => {
                self.finish_attempt_telemetry(&execution_context, ExecutionStatus::Succeeded, None)
            }
            CoordinationExecutionResult::Failed { kind, .. } => {
                let status = if *kind == CoordinationFailureKind::Cancelled {
                    ExecutionStatus::Cancelled
                } else {
                    ExecutionStatus::Failed
                };
                self.finish_attempt_telemetry(
                    &execution_context,
                    status,
                    Some(match kind {
                        CoordinationFailureKind::Retryable => "coordination_retryable",
                        CoordinationFailureKind::NonRetryable => "coordination_non_retryable",
                        CoordinationFailureKind::Cancelled => "coordination_cancelled",
                    }),
                );
            }
        }
        Ok(result)
    }

    fn cancel(&self, run_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.cancelled
            .lock()
            .map_err(executor_error)?
            .insert(run_id.to_string());
        let process_id = self.active.lock().map_err(executor_error)?.remove(run_id);
        if let Some(process_id) = process_id {
            let _ = self
                .processes
                .stop_generation(&process_id, ProcessStopInitiator::User)?;
        }
        Ok(())
    }

    fn finish_coordination(
        &self,
        run_id: &str,
        status: CoordinationRunStatus,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let context = self
            .execution_contexts
            .lock()
            .map_err(executor_error)?
            .remove(run_id);
        let Some(context) = context else {
            return Ok(());
        };
        let status = match status {
            CoordinationRunStatus::Succeeded => ExecutionStatus::Succeeded,
            CoordinationRunStatus::Cancelled => ExecutionStatus::Cancelled,
            CoordinationRunStatus::Failed => ExecutionStatus::Failed,
            CoordinationRunStatus::Queued | CoordinationRunStatus::Running => {
                ExecutionStatus::Incomplete
            }
        };
        let ended_at = chrono::Utc::now().to_rfc3339();
        let error = (status != ExecutionStatus::Succeeded).then_some("coordination_terminal");
        let _ =
            self.telemetry
                .finish_span(&context.run_id, &context.span_id, status, &ended_at, error);
        let _ = self
            .telemetry
            .finish_run(&context.run_id, status, &ended_at, error);
        let outcome = match status {
            ExecutionStatus::Succeeded => "succeeded",
            ExecutionStatus::Cancelled => "cancelled",
            ExecutionStatus::Accepted
            | ExecutionStatus::Running
            | ExecutionStatus::Failed
            | ExecutionStatus::Incomplete => "failed",
        };
        let _ = self.telemetry.add_metric(
            "vanehub.coordination.run.completed",
            1,
            &[("outcome", outcome)],
        );
        Ok(())
    }
}

struct CoordinationProcessSink {
    sender: Mutex<mpsc::Sender<GenerationProcessEvent>>,
}

impl AgentProcessEventSink for CoordinationProcessSink {
    fn handle(&self, event: GenerationProcessEvent) -> Result<(), AgentRuntimeApplicationError> {
        self.sender
            .lock()
            .map_err(executor_error)?
            .send(event)
            .map_err(executor_error)
    }
}

fn retryable(error: String) -> CoordinationExecutionResult {
    CoordinationExecutionResult::Failed {
        kind: CoordinationFailureKind::Retryable,
        error,
    }
}

fn non_retryable(error: String) -> CoordinationExecutionResult {
    CoordinationExecutionResult::Failed {
        kind: CoordinationFailureKind::NonRetryable,
        error,
    }
}

fn classify_application_error(error: AgentRuntimeApplicationError) -> CoordinationExecutionResult {
    let kind = match error {
        AgentRuntimeApplicationError::AgentUnavailable(_)
        | AgentRuntimeApplicationError::Process(_)
        | AgentRuntimeApplicationError::VerificationProcess(_)
        | AgentRuntimeApplicationError::Event(_)
        | AgentRuntimeApplicationError::Generation(_) => CoordinationFailureKind::Retryable,
        AgentRuntimeApplicationError::Domain(_)
        | AgentRuntimeApplicationError::Validation(_)
        | AgentRuntimeApplicationError::AgentNotFound(_)
        | AgentRuntimeApplicationError::SessionNotFound(_)
        | AgentRuntimeApplicationError::MessageNotFound(_)
        | AgentRuntimeApplicationError::NoActiveAgent
        | AgentRuntimeApplicationError::UnsupportedInteractionMode(_)
        | AgentRuntimeApplicationError::GenerationConflict(_)
        | AgentRuntimeApplicationError::PolicyDenied { .. }
        | AgentRuntimeApplicationError::Registry(_)
        | AgentRuntimeApplicationError::Workflow(_)
        | AgentRuntimeApplicationError::Session(_)
        | AgentRuntimeApplicationError::CliProfile(_)
        | AgentRuntimeApplicationError::Prompt(_)
        | AgentRuntimeApplicationError::Operation(_)
        | AgentRuntimeApplicationError::Loop(_)
        | AgentRuntimeApplicationError::Coordination(_)
        | AgentRuntimeApplicationError::VerificationPolicy(_)
        | AgentRuntimeApplicationError::Logging(_) => CoordinationFailureKind::NonRetryable,
    };
    CoordinationExecutionResult::Failed {
        kind,
        error: error.to_string(),
    }
}

fn cancelled_result() -> CoordinationExecutionResult {
    CoordinationExecutionResult::Failed {
        kind: CoordinationFailureKind::Cancelled,
        error: "Coordination was cancelled.".to_string(),
    }
}

#[derive(Default)]
struct BoundedOutputAccumulator {
    content: String,
    byte_count: usize,
    truncated: bool,
}

impl BoundedOutputAccumulator {
    fn push(&mut self, chunk: &str) {
        self.byte_count = self.byte_count.saturating_add(chunk.len());
        if self.truncated {
            return;
        }
        let remaining = COORDINATION_OUTPUT_LIMIT_BYTES.saturating_sub(self.content.len());
        if chunk.len() <= remaining {
            self.content.push_str(chunk);
            return;
        }
        let mut end = remaining.min(chunk.len());
        while end > 0 && !chunk.is_char_boundary(end) {
            end -= 1;
        }
        self.content.push_str(&chunk[..end]);
        self.truncated = true;
    }

    fn finish(self) -> CoordinationExecutionOutput {
        CoordinationExecutionOutput {
            content: self.content,
            byte_count: self.byte_count,
            truncated: self.truncated,
        }
    }
}

fn executor_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Coordination(error.to_string())
}

fn safe_attributes(
    entries: impl IntoIterator<Item = (String, SafeAttributeValue)>,
) -> SafeAttributes {
    SafeAttributes::try_from_entries(entries).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_error_classification_keeps_policy_and_configuration_non_retryable() {
        for error in [
            AgentRuntimeApplicationError::PolicyDenied {
                session_id: "session".to_string(),
                action: "execute".to_string(),
            },
            AgentRuntimeApplicationError::CliProfile("invalid configuration".to_string()),
            AgentRuntimeApplicationError::Validation("invalid request".to_string()),
        ] {
            assert!(matches!(
                classify_application_error(error),
                CoordinationExecutionResult::Failed {
                    kind: CoordinationFailureKind::NonRetryable,
                    ..
                }
            ));
        }
        assert!(matches!(
            classify_application_error(AgentRuntimeApplicationError::Process(
                "spawn failed".to_string()
            )),
            CoordinationExecutionResult::Failed {
                kind: CoordinationFailureKind::Retryable,
                ..
            }
        ));
    }

    #[test]
    fn streaming_output_is_bounded_without_splitting_utf8_and_tracks_original_bytes() {
        let mut output = BoundedOutputAccumulator::default();
        output.push(&"x".repeat(COORDINATION_OUTPUT_LIMIT_BYTES - 1));
        output.push("界");
        output.push("tail");

        let output = output.finish();
        assert_eq!(output.content.len(), COORDINATION_OUTPUT_LIMIT_BYTES - 1);
        assert!(output.content.is_char_boundary(output.content.len()));
        assert_eq!(
            output.byte_count,
            COORDINATION_OUTPUT_LIMIT_BYTES - 1 + "界".len() + "tail".len()
        );
        assert!(output.truncated);
    }
}
