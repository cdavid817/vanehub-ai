use super::{
    AgentChatConfiguration, AgentCliProfileGateway, AgentClockPort, AgentEvent, AgentEventPort,
    AgentGenerationPort, AgentLog, AgentLogLevel, AgentLoggingPort, AgentMessage,
    AgentProcessEventSink, AgentProcessGateway, AgentRegistryRepository,
    AgentRuntimeApplicationError, AgentSession, AgentSessionDetails, AgentSessionGateway,
    AgentTaskPort, AgentUsageRecord, AgentView, CompleteAgentMessage, EffectivePromptGateway,
    GenerationLease, GenerationProcessEvent, GenerationProcessRequest, LaunchWorkflowResult,
    LoopGenerationControlPort, LoopRoleGenerationCompletionPort, LoopRoleGenerationOutcome,
    LoopRoleGenerationTerminal, LoopVerifierGenerationPort, LoopWorkerGenerationPort,
    MessageTokenUsage, NewAgentMessage, ReadinessView, SendMessageRequest, StopGenerationResult,
    ToolLifecycleEvent, ToolLifecyclePhase, WorkflowLaunchRequest, WorkflowView,
};
use crate::contexts::agent_runtime::domain::{
    AgentDefinition, AgentLifecycle, AgentReadiness, AgentWorkflow, InteractionMode,
};
use crate::contexts::execution_observability::api::{
    ExecutionContext, ExecutionFidelity, ExecutionIdentityPort, ExecutionLink, ExecutionRun,
    ExecutionRunId, ExecutionSettingsPort, ExecutionSource, ExecutionSpan, ExecutionStatus,
    ExecutionTelemetryPort, SafeAttributeValue, SafeAttributes, SpanId, TraceId,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub(crate) struct AgentRuntimeApplicationPorts {
    pub(crate) registry: Arc<dyn AgentRegistryRepository>,
    pub(crate) workflows: Arc<dyn super::AgentWorkflowRepository>,
    pub(crate) sessions: Arc<dyn AgentSessionGateway>,
    pub(crate) cli_profiles: Arc<dyn AgentCliProfileGateway>,
    pub(crate) prompts: Arc<dyn EffectivePromptGateway>,
    pub(crate) processes: Arc<dyn AgentProcessGateway>,
    pub(crate) operations: Arc<dyn AgentTaskPort>,
    pub(crate) logging: Arc<dyn AgentLoggingPort>,
    pub(crate) clock: Arc<dyn AgentClockPort>,
    pub(crate) events: Arc<dyn AgentEventPort>,
    pub(crate) generations: Arc<dyn AgentGenerationPort>,
    pub(crate) execution_ids: Arc<dyn ExecutionIdentityPort>,
    pub(crate) execution_settings: Arc<dyn ExecutionSettingsPort>,
    pub(crate) telemetry: Arc<dyn ExecutionTelemetryPort>,
    pub(crate) loop_completions: Arc<dyn LoopRoleGenerationCompletionPort>,
}

#[derive(Clone)]
pub(crate) struct AgentRuntimeApplicationService {
    ports: AgentRuntimeApplicationPorts,
}

struct MessageGenerationInput {
    source: super::AgentMessageSource,
    configuration: AgentChatConfiguration,
    content: String,
    file_references: Vec<super::AgentFileReference>,
}

struct GenerationFailure {
    safe_error: String,
    diagnostic: String,
}

fn generation_failure(
    safe_error: impl Into<String>,
    diagnostic: impl Into<String>,
) -> GenerationFailure {
    GenerationFailure {
        safe_error: safe_error.into(),
        diagnostic: diagnostic.into(),
    }
}

impl AgentRuntimeApplicationService {
    pub(crate) fn new(ports: AgentRuntimeApplicationPorts) -> Self {
        Self { ports }
    }

    #[cfg(test)]
    pub(crate) fn take_loop_role_completion(
        &self,
        session_id: &str,
    ) -> Result<Option<LoopRoleGenerationTerminal>, AgentRuntimeApplicationError> {
        self.ports.loop_completions.take_for_session(session_id)
    }

    fn start_loop_role_generation(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<String, AgentRuntimeApplicationError> {
        let session = self.require_session(session_id)?;
        if session.loop_ownership.is_none() {
            return Err(AgentRuntimeApplicationError::Validation(
                "Loop role generation requires an owned role session.".to_string(),
            ));
        }
        let message = self.send_message(SendMessageRequest {
            session_id: session_id.to_string(),
            content: prompt.to_string(),
            source: super::AgentMessageSource::Desktop,
            configuration: AgentChatConfiguration {
                agent_id: session.agent_id,
                interaction_mode: InteractionMode::Cli,
                permission_mode: "default".to_string(),
                provider_id: None,
                model_id: None,
                reasoning_depth: None,
                streaming: true,
                thinking: false,
                long_context: false,
            },
            file_references: Vec::new(),
        })?;
        Ok(message.id)
    }

    pub(crate) fn list_agents(
        &self,
        capability_tag: Option<&str>,
    ) -> Result<Vec<AgentView>, AgentRuntimeApplicationError> {
        let agents = self.ports.registry.list()?;
        Ok(agents
            .iter()
            .filter(|agent| {
                capability_tag
                    .map(|tag| agent.has_capability(tag))
                    .unwrap_or(true)
            })
            .map(AgentView::from)
            .collect())
    }

    pub(crate) fn get_agent(
        &self,
        agent_id: &str,
    ) -> Result<AgentView, AgentRuntimeApplicationError> {
        let agent = self.require_agent(agent_id)?;
        Ok(AgentView::from(&agent))
    }

    pub(crate) fn workflow(&self) -> Result<WorkflowView, AgentRuntimeApplicationError> {
        Ok(WorkflowView::from(&self.ports.workflows.load()?))
    }

    pub(crate) fn select_agent(
        &self,
        agent_id: &str,
        interaction_mode: InteractionMode,
    ) -> Result<WorkflowView, AgentRuntimeApplicationError> {
        let agent = self.require_agent(agent_id)?;
        let current = self.ports.workflows.load()?;
        let mut workflow = AgentWorkflow::rehydrate(
            current
                .active_agent_id()
                .map(|active| active.as_str().to_string()),
            current.active_interaction_mode(),
            current.lifecycle(),
            current.intent().to_string(),
        )?;
        workflow.select(&agent, interaction_mode)?;
        self.ports.workflows.save(&workflow)?;
        let view = WorkflowView::from(&workflow);
        let _ = self
            .ports
            .events
            .publish(AgentEvent::WorkflowChanged(view.clone()));
        Ok(view)
    }

    pub(crate) fn browser_readiness(
        &self,
        agent_id: &str,
    ) -> Result<ReadinessView, AgentRuntimeApplicationError> {
        let agent = self.require_agent(agent_id)?;
        Ok(AgentReadiness::for_browser(&agent).into())
    }

    pub(crate) fn session_details(
        &self,
    ) -> Result<AgentSessionDetails, AgentRuntimeApplicationError> {
        let workflow = self.workflow()?;
        let (adapter, details) = self.ports.workflows.load_details()?;
        Ok(AgentSessionDetails {
            workflow,
            adapter,
            details,
        })
    }

    pub(crate) fn launch_active_workflow(
        &self,
    ) -> Result<LaunchWorkflowResult, AgentRuntimeApplicationError> {
        let mut workflow = self.ports.workflows.load()?;
        let agent_id = workflow
            .active_agent_id()
            .map(|value| value.as_str().to_string())
            .ok_or(AgentRuntimeApplicationError::NoActiveAgent)?;
        let interaction_mode = workflow
            .active_interaction_mode()
            .ok_or(AgentRuntimeApplicationError::NoActiveAgent)?;
        let agent = self.require_agent(&agent_id)?;
        let operation = self
            .ports
            .operations
            .start_agent_launch(&agent_id, &format!("Launching {}", agent.display_name()))?;

        workflow.begin_launch()?;
        self.ports.workflows.save(&workflow)?;
        let launch = self.ports.processes.launch_workflow(WorkflowLaunchRequest {
            operation_id: operation.id.clone(),
            agent: AgentView::from(&agent),
            interaction_mode,
        });
        let outcome = match launch {
            Ok(outcome) => outcome,
            Err(error) => {
                let _ = workflow.mark_failed();
                let _ = self.ports.workflows.save(&workflow);
                let _ = self.ports.operations.fail(&operation.id, error.to_string());
                self.record_log(
                    AgentLogLevel::Error,
                    "agent.launch",
                    error.to_string(),
                    Some(&agent_id),
                    None,
                    Some(&operation.id),
                );
                return Err(error);
            }
        };

        self.ports
            .workflows
            .save_details(&outcome.adapter, &outcome.message)?;
        workflow.mark_running()?;
        self.ports.workflows.save(&workflow)?;
        let _ = self
            .ports
            .operations
            .append_log(&operation.id, outcome.message.clone());
        let _ = self.ports.operations.complete(&operation.id);
        self.record_log(
            AgentLogLevel::Info,
            "agent.launch",
            outcome.message.clone(),
            Some(&agent_id),
            None,
            Some(&operation.id),
        );
        let workflow = WorkflowView::from(&workflow);
        let _ = self
            .ports
            .events
            .publish(AgentEvent::WorkflowChanged(workflow.clone()));
        Ok(LaunchWorkflowResult {
            operation_id: operation.id,
            workflow,
            message: outcome.message,
        })
    }

    pub(crate) fn send_message(
        &self,
        request: SendMessageRequest,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        let content = request.content.trim().to_string();
        if content.is_empty() {
            return Err(AgentRuntimeApplicationError::Validation(
                "Message content cannot be empty.".to_string(),
            ));
        }
        let session = self.require_session(&request.session_id)?;
        if session.archived {
            return Err(AgentRuntimeApplicationError::Validation(
                "Archived sessions cannot accept messages.".to_string(),
            ));
        }
        let mut configuration = self
            .ports
            .sessions
            .validate_configuration(&session, request.configuration)?;
        if session.read_only {
            configuration.permission_mode = "plan".to_string();
        }
        let agent = self.require_agent(&session.agent_id)?;
        if !agent.supports(configuration.interaction_mode) {
            return Err(AgentRuntimeApplicationError::UnsupportedInteractionMode(
                configuration.interaction_mode.as_str().to_string(),
            ));
        }
        let lease = self.ports.generations.reserve(&session.id)?;
        let result = self.start_message_generation(
            &session,
            &agent,
            MessageGenerationInput {
                source: request.source,
                configuration,
                content,
                file_references: request.file_references,
            },
            &lease,
        );
        if result.is_err() {
            let _ = self.ports.generations.release(&lease);
        }
        result
    }

    fn start_message_generation(
        &self,
        session: &AgentSession,
        agent: &AgentDefinition,
        input: MessageGenerationInput,
        lease: &GenerationLease,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        let MessageGenerationInput {
            source,
            configuration,
            content,
            file_references,
        } = input;
        let settings = self.ports.execution_settings.load_settings().map_err(|_| {
            AgentRuntimeApplicationError::Process(
                "execution observability settings are unavailable".to_string(),
            )
        })?;
        let root_context = self.ports.execution_ids.next_context(
            settings.capture_policy,
            settings.sampling_ratio,
            settings.mcp_relay_enabled,
        );
        let started_at = self.ports.clock.now();
        let mut run = ExecutionRun {
            context: root_context.clone(),
            source: execution_source(source),
            status: ExecutionStatus::Running,
            started_at: started_at.clone(),
            ended_at: None,
            error_classification: None,
            session_id: Some(session.id.clone()),
            user_message_id: None,
            assistant_message_id: None,
            operation_id: None,
            agent_id: Some(agent.id().as_str().to_string()),
            provider_session_id: session.runtime_session_id.clone(),
            attributes: safe_attributes([
                (
                    "vanehub.stage".to_string(),
                    SafeAttributeValue::String("task_execution".to_string()),
                ),
                (
                    "vanehub.agent.id".to_string(),
                    SafeAttributeValue::String(agent.id().as_str().to_string()),
                ),
            ]),
            links: Vec::new(),
        };
        let root_span = ExecutionSpan {
            context: root_context.clone(),
            parent_span_id: None,
            name: "vanehub.task.execute".to_string(),
            status: ExecutionStatus::Running,
            fidelity: ExecutionFidelity::Native,
            started_at: started_at.clone(),
            ended_at: None,
            error_classification: None,
            attributes: safe_attributes([
                (
                    "vanehub.stage".to_string(),
                    SafeAttributeValue::String("task_execution".to_string()),
                ),
                (
                    "vanehub.agent.id".to_string(),
                    SafeAttributeValue::String(agent.id().as_str().to_string()),
                ),
            ]),
            links: Vec::new(),
        };
        let _ = self.ports.telemetry.start_run(&run);
        let _ = self.ports.telemetry.start_span(&root_span);
        if let Err(error) = self.ports.generations.correlate(lease, &root_context) {
            self.finish_execution_root(
                &root_context,
                ExecutionStatus::Failed,
                Some("generation_correlation_failed"),
            );
            return Err(error);
        }

        let prompt_context = child_context(&root_context, self.ports.execution_ids.next_span_id());
        let prompt_span = ExecutionSpan {
            context: prompt_context.clone(),
            parent_span_id: Some(root_context.span_id.clone()),
            name: "vanehub.prompt.assemble".to_string(),
            status: ExecutionStatus::Running,
            fidelity: ExecutionFidelity::Native,
            started_at: self.ports.clock.now(),
            ended_at: None,
            error_classification: None,
            attributes: safe_attributes([(
                "vanehub.stage".to_string(),
                SafeAttributeValue::String("prompt_assembly".to_string()),
            )]),
            links: Vec::new(),
        };
        let _ = self.ports.telemetry.start_span(&prompt_span);
        let prompt =
            match self
                .ports
                .sessions
                .compose_prompt(&session.id, &content, &file_references)
            {
                Ok(prompt) => prompt,
                Err(error) => {
                    let ended_at = self.ports.clock.now();
                    let _ = self.ports.telemetry.finish_span(
                        &prompt_context.run_id,
                        &prompt_context.span_id,
                        ExecutionStatus::Failed,
                        &ended_at,
                        Some("prompt_compose_failed"),
                    );
                    self.finish_execution_root(
                        &root_context,
                        ExecutionStatus::Failed,
                        Some("prompt_compose_failed"),
                    );
                    return Err(error);
                }
            };
        let _ = self.ports.telemetry.finish_span(
            &prompt_context.run_id,
            &prompt_context.span_id,
            ExecutionStatus::Succeeded,
            &self.ports.clock.now(),
            None,
        );
        let user_message = match self.ports.sessions.create_message(NewAgentMessage {
            session_id: session.id.clone(),
            role: "user".to_string(),
            status: "completed".to_string(),
            content,
            file_references,
        }) {
            Ok(message) => message,
            Err(error) => {
                self.finish_execution_root(
                    &root_context,
                    ExecutionStatus::Failed,
                    Some("user_message_persistence_failed"),
                );
                return Err(error);
            }
        };
        let assistant = match self.ports.sessions.create_message(NewAgentMessage {
            session_id: session.id.clone(),
            role: "assistant".to_string(),
            status: "streaming".to_string(),
            content: String::new(),
            file_references: Vec::new(),
        }) {
            Ok(message) => message,
            Err(error) => {
                self.finish_execution_root(
                    &root_context,
                    ExecutionStatus::Failed,
                    Some("assistant_message_persistence_failed"),
                );
                return Err(error);
            }
        };
        let operation = match self.ports.operations.start_agent_generation(
            agent.id().as_str(),
            &session.id,
            &assistant.id,
        ) {
            Ok(operation) => operation,
            Err(error) => {
                return self.fail_prepared_message(
                    &root_context,
                    session,
                    &assistant,
                    lease,
                    None,
                    generation_failure(
                        format!("{} command failed", agent.display_name()),
                        error.to_string(),
                    ),
                );
            }
        };
        run.user_message_id = Some(user_message.id);
        run.assistant_message_id = Some(assistant.id.clone());
        run.operation_id = Some(operation.id.clone());
        let _ = self.ports.telemetry.start_run(&run);
        let _ = self.ports.operations.correlate_execution(
            &operation.id,
            root_context.run_id.as_str(),
            root_context.trace_id.as_str(),
        );
        if let Err(error) = self
            .ports
            .sessions
            .update_lifecycle(&session.id, AgentLifecycle::Starting)
        {
            return self.fail_prepared_message(
                &root_context,
                session,
                &assistant,
                lease,
                Some(&operation.id),
                generation_failure(
                    format!("{} command failed", agent.display_name()),
                    error.to_string(),
                ),
            );
        }
        let _ = self.ports.events.publish(AgentEvent::MessageStarted {
            session_id: session.id.clone(),
            message_id: assistant.id.clone(),
        });
        if let Err(error) = self
            .ports
            .sessions
            .update_lifecycle(&session.id, AgentLifecycle::Running)
        {
            return self.fail_prepared_message(
                &root_context,
                session,
                &assistant,
                lease,
                Some(&operation.id),
                generation_failure(
                    format!("{} command failed", agent.display_name()),
                    error.to_string(),
                ),
            );
        }

        let effective_prompt =
            match self
                .ports
                .prompts
                .assemble(agent.id().as_str(), &session.id, &prompt)
            {
                Ok(prompt) => prompt,
                Err(error) => {
                    return self.fail_prepared_message(
                        &root_context,
                        session,
                        &assistant,
                        lease,
                        Some(&operation.id),
                        generation_failure("Prompt Hook assembly failed", error.to_string()),
                    );
                }
            };
        for trace in &effective_prompt.trace {
            self.record_log(
                AgentLogLevel::Debug,
                "session.runtime.prompt-hook",
                format!(
                    "Prompt Hook {} {} hash={} tokens={} reason={}",
                    trace.hook_id,
                    trace.status,
                    trace.content_hash.as_deref().unwrap_or("none"),
                    trace
                        .token_estimate
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "none".to_string()),
                    trace.reason.as_deref().unwrap_or("none")
                ),
                Some(agent.id().as_str()),
                Some(&session.id),
                None,
            );
        }
        let profile = match self
            .ports
            .cli_profiles
            .load(agent.id().as_str(), &configuration)
        {
            Ok(profile) => profile,
            Err(error) => {
                return self.fail_prepared_message(
                    &root_context,
                    session,
                    &assistant,
                    lease,
                    Some(&operation.id),
                    generation_failure(
                        format!("{} command failed", agent.display_name()),
                        error.to_string(),
                    ),
                );
            }
        };
        let input_count = effective_prompt.content.chars().count();
        let agent_context = child_context(&root_context, self.ports.execution_ids.next_span_id());
        let mut agent_attributes = vec![
            (
                "gen_ai.operation.name".to_string(),
                SafeAttributeValue::String("invoke_agent".to_string()),
            ),
            (
                "vanehub.agent.id".to_string(),
                SafeAttributeValue::String(agent.id().as_str().to_string()),
            ),
        ];
        if let Some(provider_id) = &configuration.provider_id {
            agent_attributes.push((
                "gen_ai.provider.name".to_string(),
                SafeAttributeValue::String(provider_id.clone()),
            ));
        }
        if let Some(model_id) = &configuration.model_id {
            agent_attributes.push((
                "gen_ai.request.model".to_string(),
                SafeAttributeValue::String(model_id.clone()),
            ));
        }
        let agent_span = ExecutionSpan {
            context: agent_context.clone(),
            parent_span_id: Some(root_context.span_id.clone()),
            name: format!("invoke_agent {}", agent.id().as_str()),
            status: ExecutionStatus::Running,
            fidelity: ExecutionFidelity::Native,
            started_at: self.ports.clock.now(),
            ended_at: None,
            error_classification: None,
            attributes: safe_attributes(agent_attributes),
            links: Vec::new(),
        };
        let _ = self.ports.telemetry.start_span(&agent_span);
        let started = match self
            .ports
            .processes
            .start_generation(GenerationProcessRequest {
                execution_context: agent_context.clone(),
                session: session.clone(),
                agent: AgentView::from(agent),
                message_id: assistant.id.clone(),
                operation_id: operation.id.clone(),
                configuration: configuration.clone(),
                effective_prompt: effective_prompt.content,
                cli_profile: profile,
            }) {
            Ok(started) => started,
            Err(error) => {
                let _ = self.ports.telemetry.finish_span(
                    &agent_context.run_id,
                    &agent_context.span_id,
                    ExecutionStatus::Failed,
                    &self.ports.clock.now(),
                    Some("process_start_failed"),
                );
                return self.fail_prepared_message(
                    &root_context,
                    session,
                    &assistant,
                    lease,
                    Some(&operation.id),
                    generation_failure(
                        format!("{} command failed", agent.display_name()),
                        error.to_string(),
                    ),
                );
            }
        };
        if let Err(error) =
            self.ports
                .generations
                .attach(lease, &assistant.id, &started.process_id, &operation.id)
        {
            let _ = self.ports.processes.stop_generation(
                &started.process_id,
                super::ProcessStopInitiator::RuntimeCleanup,
            );
            let _ = self.ports.telemetry.finish_span(
                &agent_context.run_id,
                &agent_context.span_id,
                ExecutionStatus::Failed,
                &self.ports.clock.now(),
                Some("generation_attach_failed"),
            );
            return self.fail_prepared_message(
                &root_context,
                session,
                &assistant,
                lease,
                Some(&operation.id),
                generation_failure(
                    format!("{} command failed", agent.display_name()),
                    error.to_string(),
                ),
            );
        }
        let sink: Arc<dyn AgentProcessEventSink> = Arc::new(GenerationEventHandler::new(
            self.ports.clone(),
            GenerationEventHandlerInput {
                session_id: session.id.clone(),
                agent_id: agent.id().as_str().to_string(),
                message_id: assistant.id.clone(),
                operation_id: operation.id.clone(),
                safe_error: format!("{} command failed", agent.display_name()),
                configuration,
                input_count,
                root_context: root_context.clone(),
                agent_context: agent_context.clone(),
                loop_ownership: session.loop_ownership.clone(),
            },
        ));
        if let Err(error) = self
            .ports
            .processes
            .monitor_generation(&started.process_id, sink)
        {
            let _ = self.ports.processes.stop_generation(
                &started.process_id,
                super::ProcessStopInitiator::RuntimeCleanup,
            );
            let _ = self.ports.telemetry.finish_span(
                &agent_context.run_id,
                &agent_context.span_id,
                ExecutionStatus::Failed,
                &self.ports.clock.now(),
                Some("generation_monitor_failed"),
            );
            return self.fail_prepared_message(
                &root_context,
                session,
                &assistant,
                lease,
                Some(&operation.id),
                generation_failure(
                    format!("{} command failed", agent.display_name()),
                    error.to_string(),
                ),
            );
        }
        Ok(assistant)
    }

    fn fail_prepared_message(
        &self,
        execution_context: &ExecutionContext,
        session: &AgentSession,
        assistant: &AgentMessage,
        lease: &GenerationLease,
        operation_id: Option<&str>,
        failure: GenerationFailure,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        self.finish_execution_root(
            execution_context,
            ExecutionStatus::Failed,
            Some("agent_generation_failed"),
        );
        self.record_log(
            AgentLogLevel::Error,
            "session.runtime",
            failure.diagnostic,
            Some(&session.agent_id),
            Some(&session.id),
            operation_id,
        );
        let failed =
            self.ports
                .sessions
                .fail_message(&assistant.id, &session.id, &failure.safe_error)?;
        self.ports
            .sessions
            .update_lifecycle(&session.id, AgentLifecycle::Failed)?;
        let _ = self.ports.generations.release(lease);
        if let Some(operation_id) = operation_id {
            let _ = self
                .ports
                .operations
                .fail(operation_id, failure.safe_error.clone());
        }
        let _ = self.ports.events.publish(AgentEvent::MessageFailed {
            session_id: session.id.clone(),
            message_id: assistant.id.clone(),
            error: failure.safe_error,
        });
        self.deliver_loop_terminal(
            session,
            &assistant.id,
            LoopRoleGenerationOutcome::Failed,
            None,
            Some(format!("{} command failed", session.agent_id)),
        )?;
        Ok(failed)
    }

    fn finish_execution_root(
        &self,
        context: &ExecutionContext,
        status: ExecutionStatus,
        error_classification: Option<&str>,
    ) {
        let ended_at = self.ports.clock.now();
        let _ = self.ports.telemetry.finish_span(
            &context.run_id,
            &context.span_id,
            status,
            &ended_at,
            error_classification,
        );
        let _ = self.ports.telemetry.finish_run(
            &context.run_id,
            status,
            &ended_at,
            error_classification,
        );
    }

    pub(crate) fn stop_generation(
        &self,
        session_id: &str,
    ) -> Result<StopGenerationResult, AgentRuntimeApplicationError> {
        let session = self.require_session(session_id)?;
        let cancellation = self.ports.generations.cancel(session_id)?;
        let streaming_ids = self.ports.sessions.cancel_streaming_messages(session_id)?;
        let mut message_ids = BTreeSet::new();
        if let Some(message_id) = cancellation
            .as_ref()
            .and_then(|outcome| outcome.message_id.clone())
        {
            message_ids.insert(message_id);
        }
        message_ids.extend(streaming_ids);
        let has_process = cancellation
            .as_ref()
            .and_then(|outcome| outcome.process_id.as_deref())
            .is_some();
        if message_ids.is_empty() && !has_process {
            return Ok(StopGenerationResult {
                cancelled_message_ids: Vec::new(),
                process_stopped: false,
            });
        }
        let operation_id = cancellation
            .as_ref()
            .and_then(|outcome| outcome.operation_id.as_deref());
        self.ports
            .sessions
            .update_lifecycle(session_id, AgentLifecycle::Stopped)?;
        if let Some(operation_id) = operation_id {
            let _ = self.ports.operations.cancel(operation_id);
        }
        self.record_log(
            AgentLogLevel::Warn,
            "session.runtime",
            "session generation cancelled".to_string(),
            Some(&session.agent_id),
            Some(session_id),
            operation_id,
        );
        let process_stopped = match cancellation
            .as_ref()
            .and_then(|outcome| outcome.process_id.as_deref())
        {
            Some(process_id) => self
                .ports
                .processes
                .stop_generation(process_id, super::ProcessStopInitiator::User)?,
            None => false,
        };
        if let Some(execution_context) = cancellation
            .as_ref()
            .and_then(|outcome| outcome.execution_context.as_ref())
        {
            self.finish_execution_root(
                execution_context,
                ExecutionStatus::Cancelled,
                Some("user_cancelled"),
            );
        }
        for message_id in &message_ids {
            let _ = self.ports.events.publish(AgentEvent::MessageCancelled {
                session_id: session_id.to_string(),
                message_id: message_id.clone(),
            });
            self.deliver_loop_terminal(
                &session,
                message_id,
                LoopRoleGenerationOutcome::Cancelled,
                None,
                None,
            )?;
        }
        Ok(StopGenerationResult {
            cancelled_message_ids: message_ids.into_iter().collect(),
            process_stopped,
        })
    }

    fn deliver_loop_terminal(
        &self,
        session: &AgentSession,
        message_id: &str,
        outcome: LoopRoleGenerationOutcome,
        content: Option<String>,
        error: Option<String>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let Some(ownership) = &session.loop_ownership else {
            return Ok(());
        };
        self.ports
            .loop_completions
            .deliver(LoopRoleGenerationTerminal {
                run_id: ownership.run_id.clone(),
                iteration_id: ownership.iteration_id.clone(),
                role: ownership.role.clone(),
                session_id: session.id.clone(),
                message_id: message_id.to_string(),
                outcome,
                content,
                error,
            })?;
        Ok(())
    }

    fn require_agent(
        &self,
        agent_id: &str,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        self.ports
            .registry
            .find(agent_id)?
            .ok_or_else(|| AgentRuntimeApplicationError::AgentNotFound(agent_id.to_string()))
    }

    fn require_session(
        &self,
        session_id: &str,
    ) -> Result<AgentSession, AgentRuntimeApplicationError> {
        self.ports
            .sessions
            .find_session(session_id)?
            .ok_or_else(|| AgentRuntimeApplicationError::SessionNotFound(session_id.to_string()))
    }

    fn record_log(
        &self,
        level: AgentLogLevel,
        category: &str,
        message: String,
        agent_id: Option<&str>,
        session_id: Option<&str>,
        operation_id: Option<&str>,
    ) {
        let _ = self.ports.logging.record(AgentLog {
            level,
            category: category.to_string(),
            message,
            agent_id: agent_id.map(str::to_string),
            session_id: session_id.map(str::to_string),
            operation_id: operation_id.map(str::to_string),
            run_id: None,
            trace_id: None,
            span_id: None,
            occurred_at: self.ports.clock.now(),
        });
    }
}

impl LoopWorkerGenerationPort for AgentRuntimeApplicationService {
    fn start_worker_generation(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<String, AgentRuntimeApplicationError> {
        self.start_loop_role_generation(session_id, prompt)
    }
}

impl LoopVerifierGenerationPort for AgentRuntimeApplicationService {
    fn start_verifier_generation(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<String, AgentRuntimeApplicationError> {
        self.start_loop_role_generation(session_id, prompt)
    }
}

impl LoopGenerationControlPort for AgentRuntimeApplicationService {
    fn stop_loop_generation(&self, session_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.stop_generation(session_id).map(|_| ())
    }
}

struct GenerationEventHandler {
    ports: AgentRuntimeApplicationPorts,
    session_id: String,
    agent_id: String,
    message_id: String,
    operation_id: String,
    safe_error: String,
    configuration: AgentChatConfiguration,
    input_count: usize,
    root_context: ExecutionContext,
    agent_context: ExecutionContext,
    loop_ownership: Option<super::LoopRoleGenerationOwnership>,
    state: Mutex<GenerationStreamState>,
}

struct GenerationEventHandlerInput {
    session_id: String,
    agent_id: String,
    message_id: String,
    operation_id: String,
    safe_error: String,
    configuration: AgentChatConfiguration,
    input_count: usize,
    root_context: ExecutionContext,
    agent_context: ExecutionContext,
    loop_ownership: Option<super::LoopRoleGenerationOwnership>,
}

// Streaming deltas are persisted for crash/live-reload durability only — the terminal
// path rewrites the full message content anyway. Persisting every token meant an
// O(N²) load-full-row + rewrite-full-content per token; instead we coalesce deltas and
// flush at most this often (bounding the flush count by wall-clock, not token count) or
// once the un-persisted buffer grows past the byte cap.
const STREAM_FLUSH_INTERVAL: Duration = Duration::from_millis(250);
const STREAM_FLUSH_MAX_PENDING_BYTES: usize = 8 * 1024;

struct GenerationStreamState {
    response: String,
    phase: GenerationStreamPhase,
    active_tool_spans: BTreeMap<String, crate::contexts::execution_observability::api::SpanId>,
    terminal_tool_calls: BTreeSet<String>,
    pending_content: String,
    pending_thinking: String,
    last_flush: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum GenerationStreamPhase {
    #[default]
    Active,
    ApplyingTerminal,
    Terminal,
}

impl Default for GenerationStreamState {
    fn default() -> Self {
        Self {
            response: String::new(),
            phase: GenerationStreamPhase::Active,
            active_tool_spans: BTreeMap::new(),
            terminal_tool_calls: BTreeSet::new(),
            pending_content: String::new(),
            pending_thinking: String::new(),
            last_flush: Instant::now(),
        }
    }
}

impl GenerationStreamState {
    fn should_flush(&self) -> bool {
        self.last_flush.elapsed() >= STREAM_FLUSH_INTERVAL
            || self.pending_content.len() >= STREAM_FLUSH_MAX_PENDING_BYTES
            || self.pending_thinking.len() >= STREAM_FLUSH_MAX_PENDING_BYTES
    }

    fn take_pending_content(&mut self) -> String {
        self.last_flush = Instant::now();
        std::mem::take(&mut self.pending_content)
    }

    fn take_pending_thinking(&mut self) -> String {
        self.last_flush = Instant::now();
        std::mem::take(&mut self.pending_thinking)
    }
}

impl GenerationEventHandler {
    fn new(ports: AgentRuntimeApplicationPorts, input: GenerationEventHandlerInput) -> Self {
        Self {
            ports,
            session_id: input.session_id,
            agent_id: input.agent_id,
            message_id: input.message_id,
            operation_id: input.operation_id,
            safe_error: input.safe_error,
            configuration: input.configuration,
            input_count: input.input_count,
            root_context: input.root_context,
            agent_context: input.agent_context,
            loop_ownership: input.loop_ownership,
            state: Mutex::new(GenerationStreamState::default()),
        }
    }

    fn token(&self, delta: String) -> Result<(), AgentRuntimeApplicationError> {
        let (content_delta, flushed) = {
            let mut state = self.state()?;
            if state.phase != GenerationStreamPhase::Active {
                return Ok(());
            }
            let content_delta = if state.response.is_empty() {
                delta
            } else {
                format!("\n{delta}")
            };
            state.response.push_str(&content_delta);
            state.pending_content.push_str(&content_delta);
            let flushed = state.should_flush().then(|| state.take_pending_content());
            (content_delta, flushed)
        };
        // The frontend accumulates from the per-token event, so live rendering is
        // unaffected by how often we persist; the DB write is coalesced.
        if let Some(pending) = flushed {
            self.ports
                .sessions
                .append_content(&self.message_id, &pending)?;
        }
        let _ = self.ports.events.publish(AgentEvent::MessageToken {
            session_id: self.session_id.clone(),
            message_id: self.message_id.clone(),
            content_delta,
        });
        Ok(())
    }

    fn thinking(&self, content_delta: String) -> Result<(), AgentRuntimeApplicationError> {
        let flushed = {
            let mut state = self.state()?;
            if state.phase != GenerationStreamPhase::Active {
                return Ok(());
            }
            state.pending_thinking.push_str(&content_delta);
            state.should_flush().then(|| state.take_pending_thinking())
        };
        if let Some(pending) = flushed {
            self.ports
                .sessions
                .append_thinking(&self.message_id, &pending)?;
        }
        let _ = self.ports.events.publish(AgentEvent::MessageThinking {
            session_id: self.session_id.clone(),
            message_id: self.message_id.clone(),
            content_delta,
        });
        Ok(())
    }

    fn tool_use(&self, tool_use: super::ToolUseBlock) -> Result<(), AgentRuntimeApplicationError> {
        let phase = match tool_terminal_status(&tool_use.status) {
            Some(ExecutionStatus::Succeeded) => ToolLifecyclePhase::Completed,
            Some(_) => ToolLifecyclePhase::Failed,
            None => ToolLifecyclePhase::Started,
        };
        self.tool_lifecycle(ToolLifecycleEvent {
            call_id: tool_use.id.clone(),
            phase,
            provider_timestamp: None,
            fidelity: ExecutionFidelity::Inferred,
            parent_run_id: None,
            parent_trace_id: None,
            parent_span_id: None,
            delegation_id: None,
            attempt: None,
            tool_use,
        })
    }

    fn tool_lifecycle(
        &self,
        event: ToolLifecycleEvent,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let is_terminal = matches!(
            event.phase,
            ToolLifecyclePhase::Completed | ToolLifecyclePhase::Failed
        );
        let (span_id, is_new) = {
            let mut state = self.state()?;
            if state.phase != GenerationStreamPhase::Active {
                return Ok(());
            }
            if state.terminal_tool_calls.contains(&event.call_id) {
                return Ok(());
            }
            match state.active_tool_spans.get(&event.call_id) {
                Some(span_id) => (span_id.clone(), false),
                None => {
                    let span_id = self.ports.execution_ids.next_span_id();
                    state
                        .active_tool_spans
                        .insert(event.call_id.clone(), span_id.clone());
                    (span_id, true)
                }
            }
        };
        if is_new {
            let context = child_context(&self.agent_context, span_id.clone());
            let fidelity = if is_terminal {
                ExecutionFidelity::Opaque
            } else {
                event.fidelity
            };
            let mut attributes = vec![
                (
                    "gen_ai.tool.name".to_string(),
                    SafeAttributeValue::String(event.tool_use.name.clone()),
                ),
                (
                    "vanehub.tool.duration_known".to_string(),
                    SafeAttributeValue::Boolean(!is_terminal),
                ),
            ];
            if let Some(delegation_id) = &event.delegation_id {
                attributes.push((
                    "vanehub.delegation.id".to_string(),
                    SafeAttributeValue::String(delegation_id.clone()),
                ));
            }
            if let Some(attempt) = event.attempt {
                attributes.push((
                    "vanehub.execution.attempt".to_string(),
                    SafeAttributeValue::Integer(i64::from(attempt)),
                ));
            }
            let links = provider_parent_link(&event);
            let _ = self.ports.telemetry.start_span(&ExecutionSpan {
                context,
                parent_span_id: Some(self.agent_context.span_id.clone()),
                name: format!("execute_tool {}", event.tool_use.name),
                status: ExecutionStatus::Running,
                fidelity,
                started_at: event
                    .provider_timestamp
                    .clone()
                    .unwrap_or_else(|| self.ports.clock.now()),
                ended_at: None,
                error_classification: None,
                attributes: safe_attributes(attributes),
                links,
            });
        }
        if !is_new && event.phase == ToolLifecyclePhase::Started {
            return Ok(());
        }
        let terminal_status = match event.phase {
            ToolLifecyclePhase::Completed => Some(ExecutionStatus::Succeeded),
            ToolLifecyclePhase::Failed => Some(ExecutionStatus::Failed),
            ToolLifecyclePhase::Started | ToolLifecyclePhase::Updated => None,
        };
        if let Some(status) = terminal_status {
            let error_classification =
                (status == ExecutionStatus::Failed).then_some("provider_tool_failed");
            let ended_at = event
                .provider_timestamp
                .clone()
                .unwrap_or_else(|| self.ports.clock.now());
            let _ = self.ports.telemetry.finish_span(
                &self.agent_context.run_id,
                &span_id,
                status,
                &ended_at,
                error_classification,
            );
            let mut state = self.state()?;
            state.active_tool_spans.remove(&event.call_id);
            state.terminal_tool_calls.insert(event.call_id.clone());
        }
        self.ports
            .sessions
            .append_tool_use(&self.message_id, event.tool_use.clone())?;
        let _ = self.ports.events.publish(AgentEvent::MessageToolUse {
            session_id: self.session_id.clone(),
            message_id: self.message_id.clone(),
            tool_use: event.tool_use,
        });
        Ok(())
    }

    fn rich_block(&self, block: serde_json::Value) -> Result<(), AgentRuntimeApplicationError> {
        if self.state()?.phase != GenerationStreamPhase::Active {
            return Ok(());
        }
        self.ports
            .sessions
            .append_rich_block(&self.message_id, block.clone())?;
        let _ = self.ports.events.publish(AgentEvent::MessageRichBlock {
            session_id: self.session_id.clone(),
            message_id: self.message_id.clone(),
            block,
        });
        Ok(())
    }

    fn completed(&self) -> Result<(), AgentRuntimeApplicationError> {
        let Some(response) = self.begin_terminal()? else {
            return Ok(());
        };
        let result = self.complete_claimed(response);
        if result.is_err() {
            self.finish_execution(
                ExecutionStatus::Failed,
                Some("completion_persistence_failed"),
            );
        }
        self.finish_terminal(result.is_ok())?;
        result
    }

    fn complete_claimed(&self, response: String) -> Result<(), AgentRuntimeApplicationError> {
        let current = self.current_message()?;
        if current.status == "cancelled" {
            self.mark_cancelled();
            return Ok(());
        }
        let token_usage = MessageTokenUsage {
            input: bounded_count(self.input_count),
            output: bounded_count(response.chars().count()),
        };
        let usage = AgentUsageRecord {
            message_id: self.message_id.clone(),
            session_id: self.session_id.clone(),
            agent_id: self.agent_id.clone(),
            provider_id: self.configuration.provider_id.clone(),
            model_id: self.configuration.model_id.clone(),
            input_count: token_usage.input,
            output_count: token_usage.output,
            source: "character-count".to_string(),
            occurred_at: self.ports.clock.now(),
        };
        self.ports.sessions.complete_message(CompleteAgentMessage {
            message_id: self.message_id.clone(),
            session_id: self.session_id.clone(),
            content: response.clone(),
            thinking_content: current.thinking_content,
            tool_use: current.tool_use,
            rich_blocks: current.rich_blocks,
            token_usage: Some(token_usage.clone()),
            usage: Some(usage),
        })?;
        self.ports
            .sessions
            .update_lifecycle(&self.session_id, AgentLifecycle::Idle)?;
        self.ports.generations.complete(&self.session_id)?;
        let _ = self
            .ports
            .operations
            .append_log(&self.operation_id, "generation completed".to_string());
        let _ = self.ports.operations.complete(&self.operation_id);
        self.finish_execution(ExecutionStatus::Succeeded, None);
        self.record_log(AgentLogLevel::Info, "generation completed".to_string());
        let _ = self.ports.events.publish(AgentEvent::MessageCompleted {
            session_id: self.session_id.clone(),
            message_id: self.message_id.clone(),
            token_usage: Some(token_usage),
        });
        self.deliver_loop_terminal(LoopRoleGenerationOutcome::Completed, Some(response), None)?;
        Ok(())
    }

    fn failed(&self, diagnostic: String) -> Result<(), AgentRuntimeApplicationError> {
        if self.begin_terminal()?.is_none() {
            return Ok(());
        }
        let result = self.fail_claimed(diagnostic);
        if result.is_err() {
            self.finish_execution(ExecutionStatus::Failed, Some("failure_persistence_failed"));
        }
        self.finish_terminal(result.is_ok())?;
        result
    }

    fn fail_claimed(&self, diagnostic: String) -> Result<(), AgentRuntimeApplicationError> {
        let current = self.current_message()?;
        if current.status == "cancelled" {
            self.mark_cancelled();
            return Ok(());
        }
        self.record_log(AgentLogLevel::Error, diagnostic);
        self.ports
            .sessions
            .fail_message(&self.message_id, &self.session_id, &self.safe_error)?;
        self.ports
            .sessions
            .update_lifecycle(&self.session_id, AgentLifecycle::Failed)?;
        let _ = self.ports.generations.fail(&self.session_id);
        let _ = self
            .ports
            .operations
            .fail(&self.operation_id, self.safe_error.clone());
        self.finish_execution(ExecutionStatus::Failed, Some("agent_generation_failed"));
        let _ = self.ports.events.publish(AgentEvent::MessageFailed {
            session_id: self.session_id.clone(),
            message_id: self.message_id.clone(),
            error: self.safe_error.clone(),
        });
        self.deliver_loop_terminal(
            LoopRoleGenerationOutcome::Failed,
            None,
            Some(self.safe_error.clone()),
        )?;
        Ok(())
    }

    fn stderr(&self, diagnostic: String) {
        let _ = self
            .ports
            .operations
            .append_log(&self.operation_id, diagnostic.clone());
        self.record_log(AgentLogLevel::Warn, diagnostic);
    }

    fn deliver_loop_terminal(
        &self,
        outcome: LoopRoleGenerationOutcome,
        content: Option<String>,
        error: Option<String>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let Some(ownership) = &self.loop_ownership else {
            return Ok(());
        };
        self.ports
            .loop_completions
            .deliver(LoopRoleGenerationTerminal {
                run_id: ownership.run_id.clone(),
                iteration_id: ownership.iteration_id.clone(),
                role: ownership.role.clone(),
                session_id: self.session_id.clone(),
                message_id: self.message_id.clone(),
                outcome,
                content,
                error,
            })?;
        Ok(())
    }

    fn current_message(&self) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        self.ports
            .sessions
            .find_message(&self.message_id)?
            .ok_or_else(|| AgentRuntimeApplicationError::MessageNotFound(self.message_id.clone()))
    }

    fn mark_cancelled(&self) {
        let _ = self.ports.operations.cancel(&self.operation_id);
        self.finish_execution(ExecutionStatus::Cancelled, Some("user_cancelled"));
        if let Ok(mut state) = self.state() {
            state.phase = GenerationStreamPhase::Terminal;
        }
    }

    fn begin_terminal(&self) -> Result<Option<String>, AgentRuntimeApplicationError> {
        let (response, pending_content, pending_thinking) = {
            let mut state = self.state()?;
            if state.phase != GenerationStreamPhase::Active {
                return Ok(None);
            }
            state.phase = GenerationStreamPhase::ApplyingTerminal;
            (
                state.response.clone(),
                std::mem::take(&mut state.pending_content),
                std::mem::take(&mut state.pending_thinking),
            )
        };
        // Flush the coalesced tail on the way into the terminal phase. Best-effort: the
        // success path rewrites full content via `complete_message`, but the failed path
        // and `complete_message`'s read of `thinking_content` depend on these appends.
        if !pending_content.is_empty() {
            let _ = self
                .ports
                .sessions
                .append_content(&self.message_id, &pending_content);
        }
        if !pending_thinking.is_empty() {
            let _ = self
                .ports
                .sessions
                .append_thinking(&self.message_id, &pending_thinking);
        }
        Ok(Some(response))
    }

    fn finish_terminal(&self, committed: bool) -> Result<(), AgentRuntimeApplicationError> {
        let mut state = self.state()?;
        if state.phase == GenerationStreamPhase::ApplyingTerminal {
            state.phase = if committed {
                GenerationStreamPhase::Terminal
            } else {
                GenerationStreamPhase::Active
            };
        }
        Ok(())
    }

    fn record_log(&self, level: AgentLogLevel, message: String) {
        let _ = self.ports.logging.record(AgentLog {
            level,
            category: "session.runtime".to_string(),
            message,
            agent_id: Some(self.agent_id.clone()),
            session_id: Some(self.session_id.clone()),
            operation_id: Some(self.operation_id.clone()),
            run_id: Some(self.root_context.run_id.as_str().to_string()),
            trace_id: Some(self.root_context.trace_id.as_str().to_string()),
            span_id: Some(self.agent_context.span_id.as_str().to_string()),
            occurred_at: self.ports.clock.now(),
        });
    }

    fn finish_execution(&self, status: ExecutionStatus, error_classification: Option<&str>) {
        let ended_at = self.ports.clock.now();
        if let Ok(mut state) = self.state() {
            for span_id in std::mem::take(&mut state.active_tool_spans).into_values() {
                let _ = self.ports.telemetry.finish_span(
                    &self.root_context.run_id,
                    &span_id,
                    ExecutionStatus::Incomplete,
                    &ended_at,
                    Some("provider_boundary_missing"),
                );
            }
        }
        let _ = self.ports.telemetry.finish_span(
            &self.agent_context.run_id,
            &self.agent_context.span_id,
            status,
            &ended_at,
            error_classification,
        );
        let _ = self.ports.telemetry.finish_span(
            &self.root_context.run_id,
            &self.root_context.span_id,
            status,
            &ended_at,
            error_classification,
        );
        let _ = self.ports.telemetry.finish_run(
            &self.root_context.run_id,
            status,
            &ended_at,
            error_classification,
        );
    }

    fn state(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, GenerationStreamState>, AgentRuntimeApplicationError>
    {
        self.state
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Generation(error.to_string()))
    }
}

fn provider_parent_link(event: &ToolLifecycleEvent) -> Vec<ExecutionLink> {
    let (Some(run_id), Some(trace_id)) = (&event.parent_run_id, &event.parent_trace_id) else {
        return Vec::new();
    };
    let (Ok(run_id), Ok(trace_id)) = (ExecutionRunId::parse(run_id), TraceId::parse(trace_id))
    else {
        return Vec::new();
    };
    let span_id = event
        .parent_span_id
        .as_deref()
        .and_then(|value| SpanId::parse(value).ok());
    vec![ExecutionLink {
        run_id,
        trace_id,
        span_id,
        relationship: "delegated_from".to_string(),
    }]
}

impl AgentProcessEventSink for GenerationEventHandler {
    fn handle(&self, event: GenerationProcessEvent) -> Result<(), AgentRuntimeApplicationError> {
        match event {
            GenerationProcessEvent::Token(delta) => self.token(delta),
            GenerationProcessEvent::Thinking(content_delta) => self.thinking(content_delta),
            GenerationProcessEvent::ToolUse(tool_use) => self.tool_use(tool_use),
            GenerationProcessEvent::ToolLifecycle(event) => self.tool_lifecycle(event),
            GenerationProcessEvent::RichBlock(block) => self.rich_block(block),
            GenerationProcessEvent::RuntimeSessionId(runtime_session_id) => self
                .ports
                .sessions
                .update_runtime_session_id(&self.session_id, &runtime_session_id),
            GenerationProcessEvent::Stderr(diagnostic) => {
                self.stderr(diagnostic);
                Ok(())
            }
            GenerationProcessEvent::Completed => self.completed(),
            GenerationProcessEvent::Failed(diagnostic) => self.failed(diagnostic),
        }
    }
}

fn bounded_count(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn child_context(
    parent: &ExecutionContext,
    span_id: crate::contexts::execution_observability::api::SpanId,
) -> ExecutionContext {
    ExecutionContext {
        run_id: parent.run_id.clone(),
        trace_id: parent.trace_id.clone(),
        span_id,
        capture_policy: parent.capture_policy,
        sampling_per_million: parent.sampling_per_million,
        mcp_relay_enabled: parent.mcp_relay_enabled,
    }
}

fn safe_attributes(
    entries: impl IntoIterator<Item = (String, SafeAttributeValue)>,
) -> SafeAttributes {
    SafeAttributes::try_from_entries(entries).unwrap_or_default()
}

fn execution_source(source: super::AgentMessageSource) -> ExecutionSource {
    match source {
        super::AgentMessageSource::Desktop => ExecutionSource::Desktop,
        super::AgentMessageSource::InstantMessage { connector_id } => {
            ExecutionSource::InstantMessage { connector_id }
        }
        super::AgentMessageSource::Scheduled { task_id } => ExecutionSource::Scheduled { task_id },
    }
}

fn tool_terminal_status(value: &str) -> Option<ExecutionStatus> {
    match value {
        "completed" | "succeeded" | "success" => Some(ExecutionStatus::Succeeded),
        "failed" | "error" => Some(ExecutionStatus::Failed),
        "cancelled" => Some(ExecutionStatus::Cancelled),
        _ => None,
    }
}
