use super::*;
use crate::contexts::agent_runtime::domain::{
    AgentAvailability, AgentDefinition, AgentDefinitionInput, AgentLifecycle, AgentWorkflow,
    AvailabilityAssessment, InteractionMode, LaunchMetadata,
};
use crate::contexts::execution_observability::api::{
    CapturedTelemetryRecord, CapturingExecutionTelemetry, ExecutionFidelity, ExecutionSettingsPort,
    ExecutionStatus, ExecutionTelemetryError, ObservabilitySettings, RandomExecutionIdentity,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum OperationEvent {
    Started(String),
    Logged(String),
    Completed(String),
    Failed(String),
    Cancelled(String),
}

type ActiveGeneration = (
    GenerationLease,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<crate::contexts::execution_observability::api::ExecutionContext>,
);

struct FakeWorld {
    agents: Mutex<Vec<AgentDefinition>>,
    workflow: Mutex<AgentWorkflow>,
    details: Mutex<(String, BTreeMap<String, String>)>,
    sessions: Mutex<BTreeMap<String, AgentSession>>,
    messages: Mutex<BTreeMap<String, AgentMessage>>,
    created_messages: Mutex<Vec<NewAgentMessage>>,
    lifecycle_updates: Mutex<Vec<AgentLifecycle>>,
    generation_requests: Mutex<Vec<GenerationProcessRequest>>,
    generation_sinks: Mutex<BTreeMap<String, Arc<dyn AgentProcessEventSink>>>,
    loop_terminals: Mutex<Vec<LoopRoleGenerationTerminal>>,
    stopped_processes: Mutex<Vec<String>>,
    launch_failure: AtomicBool,
    prompt_failure: AtomicBool,
    events: Mutex<Vec<AgentEvent>>,
    logs: Mutex<Vec<AgentLog>>,
    operations: Mutex<Vec<OperationEvent>>,
    active_generation: Mutex<Option<ActiveGeneration>>,
    streaming_message_ids: Mutex<Vec<String>>,
    next_message_id: AtomicUsize,
}

impl FakeWorld {
    fn new(agents: Vec<AgentDefinition>) -> Self {
        let session = AgentSession {
            id: "session-1".to_string(),
            agent_id: "codex-cli".to_string(),
            interaction_mode: InteractionMode::Cli,
            lifecycle: AgentLifecycle::Idle,
            folder: Some("C:/workspace".to_string()),
            runtime_session_id: None,
            archived: false,
            read_only: false,
            loop_ownership: None,
        };
        Self {
            agents: Mutex::new(agents),
            workflow: Mutex::new(AgentWorkflow::new("build")),
            details: Mutex::new(("none".to_string(), BTreeMap::new())),
            sessions: Mutex::new(BTreeMap::from([(session.id.clone(), session)])),
            messages: Mutex::new(BTreeMap::new()),
            created_messages: Mutex::new(Vec::new()),
            lifecycle_updates: Mutex::new(Vec::new()),
            generation_requests: Mutex::new(Vec::new()),
            generation_sinks: Mutex::new(BTreeMap::new()),
            loop_terminals: Mutex::new(Vec::new()),
            stopped_processes: Mutex::new(Vec::new()),
            launch_failure: AtomicBool::new(false),
            prompt_failure: AtomicBool::new(false),
            events: Mutex::new(Vec::new()),
            logs: Mutex::new(Vec::new()),
            operations: Mutex::new(Vec::new()),
            active_generation: Mutex::new(None),
            streaming_message_ids: Mutex::new(Vec::new()),
            next_message_id: AtomicUsize::new(0),
        }
    }
}

impl AgentRegistryRepository for FakeWorld {
    fn list(&self) -> Result<Vec<AgentDefinition>, AgentRuntimeApplicationError> {
        Ok(self.agents.lock().expect("agents").clone())
    }

    fn find(
        &self,
        agent_id: &str,
    ) -> Result<Option<AgentDefinition>, AgentRuntimeApplicationError> {
        Ok(self
            .agents
            .lock()
            .expect("agents")
            .iter()
            .find(|agent| agent.id().as_str() == agent_id)
            .cloned())
    }
}

impl AgentWorkflowRepository for FakeWorld {
    fn load(&self) -> Result<AgentWorkflow, AgentRuntimeApplicationError> {
        Ok(self.workflow.lock().expect("workflow").clone())
    }

    fn save(&self, workflow: &AgentWorkflow) -> Result<(), AgentRuntimeApplicationError> {
        *self.workflow.lock().expect("workflow") = workflow.clone();
        Ok(())
    }

    fn load_details(
        &self,
    ) -> Result<(String, BTreeMap<String, String>), AgentRuntimeApplicationError> {
        Ok(self.details.lock().expect("details").clone())
    }

    fn save_details(
        &self,
        adapter: &str,
        message: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        *self.details.lock().expect("details") = (
            adapter.to_string(),
            BTreeMap::from([("message".to_string(), message.to_string())]),
        );
        Ok(())
    }
}

impl AgentSessionGateway for FakeWorld {
    fn find_session(
        &self,
        session_id: &str,
    ) -> Result<Option<AgentSession>, AgentRuntimeApplicationError> {
        Ok(self
            .sessions
            .lock()
            .expect("sessions")
            .get(session_id)
            .cloned())
    }

    fn validate_configuration(
        &self,
        session: &AgentSession,
        configuration: AgentChatConfiguration,
    ) -> Result<AgentChatConfiguration, AgentRuntimeApplicationError> {
        if configuration.agent_id != session.agent_id
            || configuration.interaction_mode != session.interaction_mode
        {
            return Err(AgentRuntimeApplicationError::Validation(
                "Chat configuration does not match the session.".to_string(),
            ));
        }
        Ok(configuration)
    }

    fn compose_prompt(
        &self,
        _session_id: &str,
        content: &str,
        file_references: &[AgentFileReference],
    ) -> Result<String, AgentRuntimeApplicationError> {
        Ok(format!("{content}\nfiles={}", file_references.len()))
    }

    fn create_message(
        &self,
        message: NewAgentMessage,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        self.created_messages
            .lock()
            .expect("created messages")
            .push(message.clone());
        let id = format!(
            "message-{}",
            self.next_message_id.fetch_add(1, Ordering::SeqCst) + 1
        );
        let record = AgentMessage {
            id: id.clone(),
            session_id: message.session_id,
            role: message.role,
            content: message.content,
            status: message.status,
            tool_use: Vec::new(),
            thinking_content: None,
            rich_blocks: Vec::new(),
            token_usage: None,
            file_references: message.file_references,
            error: None,
            created_at: "2026-07-18T12:00:00Z".to_string(),
            updated_at: "2026-07-18T12:00:00Z".to_string(),
        };
        self.messages
            .lock()
            .expect("messages")
            .insert(id, record.clone());
        Ok(record)
    }

    fn find_message(
        &self,
        message_id: &str,
    ) -> Result<Option<AgentMessage>, AgentRuntimeApplicationError> {
        Ok(self
            .messages
            .lock()
            .expect("messages")
            .get(message_id)
            .cloned())
    }

    fn append_content(
        &self,
        message_id: &str,
        content_delta: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut messages = self.messages.lock().expect("messages");
        let message = messages
            .get_mut(message_id)
            .ok_or_else(|| AgentRuntimeApplicationError::MessageNotFound(message_id.to_string()))?;
        message.content.push_str(content_delta);
        Ok(())
    }

    fn append_thinking(
        &self,
        message_id: &str,
        content_delta: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut messages = self.messages.lock().expect("messages");
        let message = messages
            .get_mut(message_id)
            .ok_or_else(|| AgentRuntimeApplicationError::MessageNotFound(message_id.to_string()))?;
        message
            .thinking_content
            .get_or_insert_default()
            .push_str(content_delta);
        Ok(())
    }

    fn append_tool_use(
        &self,
        message_id: &str,
        tool_use: ToolUseBlock,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut messages = self.messages.lock().expect("messages");
        let message = messages
            .get_mut(message_id)
            .ok_or_else(|| AgentRuntimeApplicationError::MessageNotFound(message_id.to_string()))?;
        message.tool_use.push(tool_use);
        Ok(())
    }

    fn append_rich_block(
        &self,
        message_id: &str,
        block: Value,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut messages = self.messages.lock().expect("messages");
        let message = messages
            .get_mut(message_id)
            .ok_or_else(|| AgentRuntimeApplicationError::MessageNotFound(message_id.to_string()))?;
        message.rich_blocks.push(block);
        Ok(())
    }

    fn complete_message(
        &self,
        completed: CompleteAgentMessage,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        let mut messages = self.messages.lock().expect("messages");
        let message = messages.get_mut(&completed.message_id).ok_or_else(|| {
            AgentRuntimeApplicationError::MessageNotFound(completed.message_id.clone())
        })?;
        message.status = "completed".to_string();
        message.content = completed.content;
        message.thinking_content = completed.thinking_content;
        message.tool_use = completed.tool_use;
        message.rich_blocks = completed.rich_blocks;
        message.token_usage = completed.token_usage;
        Ok(message.clone())
    }

    fn fail_message(
        &self,
        message_id: &str,
        _session_id: &str,
        error: &str,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        let mut messages = self.messages.lock().expect("messages");
        let message = messages
            .get_mut(message_id)
            .ok_or_else(|| AgentRuntimeApplicationError::MessageNotFound(message_id.to_string()))?;
        message.status = "failed".to_string();
        message.error = Some(error.to_string());
        Ok(message.clone())
    }

    fn cancel_streaming_messages(
        &self,
        _session_id: &str,
    ) -> Result<Vec<String>, AgentRuntimeApplicationError> {
        let ids = self
            .streaming_message_ids
            .lock()
            .expect("streaming ids")
            .clone();
        let mut messages = self.messages.lock().expect("messages");
        for id in &ids {
            if let Some(message) = messages.get_mut(id) {
                message.status = "cancelled".to_string();
            }
        }
        Ok(ids)
    }

    fn update_lifecycle(
        &self,
        session_id: &str,
        lifecycle: AgentLifecycle,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.lifecycle_updates
            .lock()
            .expect("lifecycle updates")
            .push(lifecycle);
        let mut sessions = self.sessions.lock().expect("sessions");
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| AgentRuntimeApplicationError::SessionNotFound(session_id.to_string()))?;
        session.lifecycle = lifecycle;
        Ok(())
    }

    fn update_runtime_session_id(
        &self,
        session_id: &str,
        runtime_session_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut sessions = self.sessions.lock().expect("sessions");
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| AgentRuntimeApplicationError::SessionNotFound(session_id.to_string()))?;
        session.runtime_session_id = Some(runtime_session_id.to_string());
        Ok(())
    }
}

impl AgentCliProfileGateway for FakeWorld {
    fn load(
        &self,
        agent_id: &str,
        _configuration: &AgentChatConfiguration,
    ) -> Result<CliProfileSnapshot, AgentRuntimeApplicationError> {
        Ok(CliProfileSnapshot {
            executable: format!("C:/bin/{agent_id}.exe"),
            selections: BTreeMap::from([(
                "model".to_string(),
                Value::String("gpt-5.5".to_string()),
            )]),
            managed_args: vec!["--model".to_string(), "gpt-5.5".to_string()],
        })
    }
}

impl EffectivePromptGateway for FakeWorld {
    fn assemble(
        &self,
        _agent_id: &str,
        _session_id: &str,
        user_prompt: &str,
    ) -> Result<EffectivePrompt, AgentRuntimeApplicationError> {
        if self.prompt_failure.load(Ordering::SeqCst) {
            return Err(AgentRuntimeApplicationError::Prompt(
                "template failed".to_string(),
            ));
        }
        Ok(EffectivePrompt {
            content: format!("effective::{user_prompt}"),
            trace: vec![PromptTrace {
                hook_id: "system-context".to_string(),
                status: "applied".to_string(),
                content_hash: Some("hash".to_string()),
                token_estimate: Some(10),
                reason: None,
            }],
        })
    }
}

impl AgentProcessGateway for FakeWorld {
    fn launch_workflow(
        &self,
        request: WorkflowLaunchRequest,
    ) -> Result<WorkflowLaunchOutcome, AgentRuntimeApplicationError> {
        if self.launch_failure.load(Ordering::SeqCst) {
            return Err(AgentRuntimeApplicationError::Process(
                "launch failed".to_string(),
            ));
        }
        Ok(WorkflowLaunchOutcome {
            adapter: request.interaction_mode.as_str().to_string(),
            message: format!("{} launched", request.agent.display_name),
        })
    }

    fn start_generation(
        &self,
        request: GenerationProcessRequest,
    ) -> Result<StartedGenerationProcess, AgentRuntimeApplicationError> {
        self.generation_requests
            .lock()
            .expect("generation requests")
            .push(request);
        Ok(StartedGenerationProcess {
            process_id: "process-1".to_string(),
        })
    }

    fn monitor_generation(
        &self,
        process_id: &str,
        sink: Arc<dyn AgentProcessEventSink>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.generation_sinks
            .lock()
            .expect("generation sinks")
            .insert(process_id.to_string(), sink);
        Ok(())
    }

    fn stop_generation(
        &self,
        process_id: &str,
        _initiator: ProcessStopInitiator,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        self.stopped_processes
            .lock()
            .expect("stopped processes")
            .push(process_id.to_string());
        Ok(true)
    }
}

impl AgentTaskPort for FakeWorld {
    fn start_agent_launch(
        &self,
        agent_id: &str,
        _message: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        self.operations
            .lock()
            .expect("operations")
            .push(OperationEvent::Started(agent_id.to_string()));
        Ok(AgentOperation {
            id: "operation-1".to_string(),
            related_agent_id: Some(agent_id.to_string()),
            message: None,
        })
    }

    fn start_agent_generation(
        &self,
        agent_id: &str,
        _session_id: &str,
        _message_id: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        self.operations
            .lock()
            .expect("operations")
            .push(OperationEvent::Started(agent_id.to_string()));
        Ok(AgentOperation {
            id: "generation-operation-1".to_string(),
            related_agent_id: Some(agent_id.to_string()),
            message: Some("Generating response".to_string()),
        })
    }

    fn start_loop_operation(
        &self,
        context: &LoopOperationContext,
        message: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        Ok(AgentOperation {
            id: format!("loop-{}", context.kind.as_str()),
            related_agent_id: Some(context.run_id.clone()),
            message: Some(message.to_string()),
        })
    }

    fn append_log(
        &self,
        operation_id: &str,
        _line: String,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.operations
            .lock()
            .expect("operations")
            .push(OperationEvent::Logged(operation_id.to_string()));
        Ok(())
    }

    fn complete(&self, operation_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.operations
            .lock()
            .expect("operations")
            .push(OperationEvent::Completed(operation_id.to_string()));
        Ok(())
    }

    fn fail(&self, operation_id: &str, _error: String) -> Result<(), AgentRuntimeApplicationError> {
        self.operations
            .lock()
            .expect("operations")
            .push(OperationEvent::Failed(operation_id.to_string()));
        Ok(())
    }

    fn cancel(&self, operation_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.operations
            .lock()
            .expect("operations")
            .push(OperationEvent::Cancelled(operation_id.to_string()));
        Ok(())
    }
}

impl AgentLoggingPort for FakeWorld {
    fn record(&self, log: AgentLog) -> Result<(), AgentRuntimeApplicationError> {
        self.logs.lock().expect("logs").push(log);
        Ok(())
    }
}

impl AgentClockPort for FakeWorld {
    fn now(&self) -> String {
        "2026-07-18T12:00:00Z".to_string()
    }
}

impl AgentEventPort for FakeWorld {
    fn publish(&self, event: AgentEvent) -> Result<(), AgentRuntimeApplicationError> {
        self.events.lock().expect("events").push(event);
        Ok(())
    }
}

impl AgentGenerationPort for FakeWorld {
    fn reserve(&self, session_id: &str) -> Result<GenerationLease, AgentRuntimeApplicationError> {
        let mut active = self.active_generation.lock().expect("active generation");
        if active.is_some() {
            return Err(AgentRuntimeApplicationError::GenerationConflict(
                session_id.to_string(),
            ));
        }
        let lease = GenerationLease {
            session_id: session_id.to_string(),
            lease_id: "lease-1".to_string(),
        };
        *active = Some((lease.clone(), None, None, None, None));
        Ok(lease)
    }

    fn correlate(
        &self,
        lease: &GenerationLease,
        execution_context: &crate::contexts::execution_observability::api::ExecutionContext,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut active = self.active_generation.lock().expect("active generation");
        let current = active.as_mut().ok_or_else(|| {
            AgentRuntimeApplicationError::Generation("reservation missing".to_string())
        })?;
        if current.0 != *lease {
            return Err(AgentRuntimeApplicationError::Generation(
                "lease mismatch".to_string(),
            ));
        }
        current.4 = Some(execution_context.clone());
        Ok(())
    }

    fn attach(
        &self,
        lease: &GenerationLease,
        message_id: &str,
        process_id: &str,
        operation_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut active = self.active_generation.lock().expect("active generation");
        let current = active.as_mut().ok_or_else(|| {
            AgentRuntimeApplicationError::Generation("reservation missing".to_string())
        })?;
        if current.0 != *lease {
            return Err(AgentRuntimeApplicationError::Generation(
                "reservation changed".to_string(),
            ));
        }
        current.1 = Some(message_id.to_string());
        current.2 = Some(process_id.to_string());
        current.3 = Some(operation_id.to_string());
        Ok(())
    }

    fn release(&self, lease: &GenerationLease) -> Result<(), AgentRuntimeApplicationError> {
        let mut active = self.active_generation.lock().expect("active generation");
        if active.as_ref().is_some_and(|current| current.0 == *lease) {
            *active = None;
        }
        Ok(())
    }

    fn cancel(
        &self,
        _session_id: &str,
    ) -> Result<Option<GenerationCancellation>, AgentRuntimeApplicationError> {
        Ok(self
            .active_generation
            .lock()
            .expect("active generation")
            .take()
            .map(
                |(_, message_id, process_id, operation_id, execution_context)| {
                    GenerationCancellation {
                        message_id,
                        process_id,
                        operation_id,
                        execution_context,
                    }
                },
            ))
    }

    fn complete(&self, _session_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        *self.active_generation.lock().expect("active generation") = None;
        Ok(())
    }

    fn fail(&self, _session_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        *self.active_generation.lock().expect("active generation") = None;
        Ok(())
    }
}

impl ExecutionSettingsPort for FakeWorld {
    fn load_settings(&self) -> Result<ObservabilitySettings, ExecutionTelemetryError> {
        Ok(ObservabilitySettings::default())
    }
}

fn agent(
    id: &str,
    display_name: &str,
    modes: Vec<InteractionMode>,
    tags: Vec<&str>,
) -> AgentDefinition {
    AgentDefinition::new(AgentDefinitionInput {
        id: id.to_string(),
        display_name: display_name.to_string(),
        provider: "provider".to_string(),
        managed_sdk_dependency_id: None,
        launch: LaunchMetadata::new(
            "cli".to_string(),
            Some(id.to_string()),
            None,
            Some(id.to_string()),
        )
        .expect("launch"),
        supported_interaction_modes: modes,
        availability: AvailabilityAssessment::new(AgentAvailability::Available, None),
        capability_tags: tags.into_iter().map(str::to_string).collect(),
    })
    .expect("agent")
}

fn chat_configuration() -> AgentChatConfiguration {
    AgentChatConfiguration {
        agent_id: "codex-cli".to_string(),
        interaction_mode: InteractionMode::Cli,
        permission_mode: "default".to_string(),
        provider_id: Some("openai".to_string()),
        model_id: Some("gpt-5-5".to_string()),
        reasoning_depth: Some("high".to_string()),
        streaming: true,
        thinking: true,
        long_context: false,
    }
}

impl LoopRoleGenerationCompletionPort for FakeWorld {
    fn deliver(
        &self,
        terminal: LoopRoleGenerationTerminal,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let mut terminals = self.loop_terminals.lock().expect("loop terminals");
        if terminals.iter().any(|existing| {
            existing.session_id == terminal.session_id && existing.message_id == terminal.message_id
        }) {
            return Ok(false);
        }
        terminals.push(terminal);
        Ok(true)
    }

    fn take_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<LoopRoleGenerationTerminal>, AgentRuntimeApplicationError> {
        let mut terminals = self.loop_terminals.lock().expect("loop terminals");
        let Some(index) = terminals
            .iter()
            .position(|terminal| terminal.session_id == session_id)
        else {
            return Ok(None);
        };
        Ok(Some(terminals.remove(index)))
    }
}

#[test]
fn verifier_generation_forces_read_only_permission_mode() {
    let world = test_world();
    world
        .sessions
        .lock()
        .expect("sessions")
        .get_mut("session-1")
        .expect("session")
        .read_only = true;
    let service = service(world.clone());

    service
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "Inspect the current implementation.".to_string(),
            file_references: Vec::new(),
            configuration: chat_configuration(),
        })
        .expect("read-only generation");

    let requests = world
        .generation_requests
        .lock()
        .expect("generation requests");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].configuration.permission_mode, "plan");
}

fn service(world: Arc<FakeWorld>) -> AgentRuntimeApplicationService {
    service_with_telemetry(world).0
}

fn service_with_telemetry(
    world: Arc<FakeWorld>,
) -> (AgentRuntimeApplicationService, CapturingExecutionTelemetry) {
    let telemetry = CapturingExecutionTelemetry::default();
    let service = AgentRuntimeApplicationService::new(AgentRuntimeApplicationPorts {
        registry: world.clone(),
        workflows: world.clone(),
        sessions: world.clone(),
        cli_profiles: world.clone(),
        prompts: world.clone(),
        processes: world.clone(),
        operations: world.clone(),
        logging: world.clone(),
        clock: world.clone(),
        events: world.clone(),
        generations: world.clone(),
        execution_ids: Arc::new(RandomExecutionIdentity),
        execution_settings: world.clone(),
        telemetry: Arc::new(telemetry.clone()),
        loop_completions: world,
    });
    (service, telemetry)
}

fn test_world() -> Arc<FakeWorld> {
    Arc::new(FakeWorld::new(vec![
        agent(
            "codex-cli",
            "Codex CLI",
            vec![InteractionMode::Cli, InteractionMode::Browser],
            vec!["coding"],
        ),
        agent(
            "research-cli",
            "Research CLI",
            vec![InteractionMode::Cli],
            vec!["research"],
        ),
    ]))
}

#[test]
fn query_selection_and_readiness_use_only_registry_workflow_and_event_ports() {
    let world = test_world();
    let service = service(world.clone());

    let coding = service.list_agents(Some("coding")).expect("list");
    assert_eq!(coding.len(), 1);
    assert_eq!(
        service.get_agent("codex-cli").expect("agent").id,
        "codex-cli"
    );
    let selected = service
        .select_agent("codex-cli", InteractionMode::Cli)
        .expect("select");
    assert_eq!(selected.active_agent_id.as_deref(), Some("codex-cli"));
    assert_eq!(selected.lifecycle, AgentLifecycle::Idle);
    let readiness = service.browser_readiness("codex-cli").expect("readiness");
    assert!(readiness.ready);
    assert!(readiness.requires_authentication);
    assert!(matches!(
        world.events.lock().expect("events").last(),
        Some(AgentEvent::WorkflowChanged(_))
    ));
    assert!(world
        .generation_requests
        .lock()
        .expect("generation requests")
        .is_empty());
    assert!(world
        .stopped_processes
        .lock()
        .expect("stopped processes")
        .is_empty());
}

#[test]
fn launch_coordinates_lifecycle_details_operations_logs_and_failure_state() {
    let world = test_world();
    let service = service(world.clone());
    service
        .select_agent("codex-cli", InteractionMode::Cli)
        .expect("select");

    let launched = service.launch_active_workflow().expect("launch");
    assert_eq!(launched.operation_id, "operation-1");
    assert_eq!(launched.workflow.lifecycle, AgentLifecycle::Running);
    assert_eq!(
        world.details.lock().expect("details").0,
        InteractionMode::Cli.as_str()
    );
    assert!(world
        .operations
        .lock()
        .expect("operations")
        .contains(&OperationEvent::Completed("operation-1".to_string())));
    assert_eq!(
        world.logs.lock().expect("logs").last().unwrap().occurred_at,
        "2026-07-18T12:00:00Z"
    );

    world.launch_failure.store(true, Ordering::SeqCst);
    assert!(matches!(
        service.launch_active_workflow(),
        Err(AgentRuntimeApplicationError::Process(_))
    ));
    assert_eq!(
        world.workflow.lock().expect("workflow").lifecycle(),
        AgentLifecycle::Failed
    );
    assert!(world
        .operations
        .lock()
        .expect("operations")
        .contains(&OperationEvent::Failed("operation-1".to_string())));
}

#[test]
fn send_message_reserves_before_writes_and_attaches_effective_prompt_process() {
    let world = test_world();
    let service = service(world.clone());
    let message = service
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "  explain this  ".to_string(),
            configuration: chat_configuration(),
            file_references: vec![AgentFileReference {
                id: "file-1".to_string(),
                path: "src/main.rs".to_string(),
                name: "main.rs".to_string(),
                size_bytes: Some(10),
                content_hash: Some("hash".to_string()),
            }],
        })
        .expect("send");

    assert_eq!(message.id, "message-2");
    assert_eq!(message.status, "streaming");
    let requests = world
        .generation_requests
        .lock()
        .expect("generation requests");
    assert_eq!(requests.len(), 1);
    assert!(requests[0]
        .effective_prompt
        .starts_with("effective::explain this"));
    assert_eq!(requests[0].cli_profile.executable, "C:/bin/codex-cli.exe");
    drop(requests);
    assert_eq!(
        *world.lifecycle_updates.lock().expect("lifecycle updates"),
        vec![AgentLifecycle::Starting, AgentLifecycle::Running]
    );
    let active = world
        .active_generation
        .lock()
        .expect("active generation")
        .clone()
        .expect("attached generation");
    assert_eq!(active.1.as_deref(), Some("message-2"));
    assert_eq!(active.2.as_deref(), Some("process-1"));
    let coordinated_context = active.4.expect("coordinated execution context");
    let process_context = world
        .generation_requests
        .lock()
        .expect("generation requests")[0]
        .execution_context
        .clone();
    assert_eq!(coordinated_context.run_id, process_context.run_id);
    assert_eq!(coordinated_context.trace_id, process_context.trace_id);
}

#[test]
fn execution_telemetry_preserves_task_agent_and_tool_topology() {
    let world = test_world();
    let (service, telemetry) = service_with_telemetry(world.clone());
    service
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "secret prompt must not be captured".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("send");
    let sink = world
        .generation_sinks
        .lock()
        .expect("generation sinks")
        .get("process-1")
        .cloned()
        .expect("sink");
    for status in ["running", "completed"] {
        sink.handle(GenerationProcessEvent::ToolUse(ToolUseBlock {
            id: "provider-call-1".to_string(),
            name: "read".to_string(),
            input: None,
            output: None,
            status: status.to_string(),
        }))
        .expect("tool lifecycle");
    }
    sink.handle(GenerationProcessEvent::Completed)
        .expect("complete");

    let records = telemetry.records().expect("telemetry records");
    let run = records
        .iter()
        .find_map(|record| match record {
            CapturedTelemetryRecord::RunStarted(run) => Some(run),
            _ => None,
        })
        .expect("run");
    let spans = records
        .iter()
        .filter_map(|record| match record {
            CapturedTelemetryRecord::SpanStarted(span) => Some(span),
            _ => None,
        })
        .collect::<Vec<_>>();
    let root = spans
        .iter()
        .find(|span| span.name == "vanehub.task.execute")
        .expect("root span");
    let prompt = spans
        .iter()
        .find(|span| span.name == "vanehub.prompt.assemble")
        .expect("prompt span");
    let agent = spans
        .iter()
        .find(|span| span.name.starts_with("invoke_agent "))
        .expect("agent span");
    let tool = spans
        .iter()
        .find(|span| span.name == "execute_tool read")
        .expect("tool span");

    assert_eq!(root.context, run.context);
    assert_eq!(prompt.parent_span_id.as_ref(), Some(&root.context.span_id));
    assert_eq!(agent.parent_span_id.as_ref(), Some(&root.context.span_id));
    assert_eq!(tool.parent_span_id.as_ref(), Some(&agent.context.span_id));
    assert_eq!(tool.fidelity, ExecutionFidelity::Inferred);
    assert!(spans
        .iter()
        .all(|span| span.context.trace_id == run.context.trace_id));
    assert!(records.iter().any(|record| matches!(
        record,
        CapturedTelemetryRecord::RunFinished {
            status: ExecutionStatus::Succeeded,
            ..
        }
    )));
    assert!(!format!("{records:?}").contains("secret prompt must not be captured"));
}

#[test]
fn normalized_tool_lifecycle_deduplicates_and_marks_missing_boundaries() {
    let world = test_world();
    let (service, telemetry) = service_with_telemetry(world.clone());
    let message = service
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "observe tools".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("send");
    let sink = world
        .generation_sinks
        .lock()
        .expect("generation sinks")
        .get("process-1")
        .cloned()
        .expect("sink");
    let event = |call_id: &str, phase: ToolLifecyclePhase, status: &str| {
        GenerationProcessEvent::ToolLifecycle(ToolLifecycleEvent {
            call_id: call_id.to_string(),
            phase,
            provider_timestamp: None,
            fidelity: ExecutionFidelity::Inferred,
            parent_run_id: None,
            parent_trace_id: None,
            parent_span_id: None,
            delegation_id: None,
            attempt: None,
            tool_use: ToolUseBlock {
                id: call_id.to_string(),
                name: "read".to_string(),
                input: None,
                output: None,
                status: status.to_string(),
            },
        })
    };

    sink.handle(event(
        "completion-only",
        ToolLifecyclePhase::Completed,
        "completed",
    ))
    .expect("completion-only");
    sink.handle(event(
        "completion-only",
        ToolLifecyclePhase::Started,
        "running",
    ))
    .expect("late start");
    sink.handle(event("duplicate", ToolLifecyclePhase::Started, "running"))
        .expect("start");
    sink.handle(event("duplicate", ToolLifecyclePhase::Started, "running"))
        .expect("duplicate start");
    sink.handle(event("duplicate", ToolLifecyclePhase::Failed, "failed"))
        .expect("failed");
    sink.handle(event("unfinished", ToolLifecyclePhase::Started, "running"))
        .expect("unfinished");
    sink.handle(GenerationProcessEvent::Completed)
        .expect("agent complete");

    let records = telemetry.records().expect("telemetry records");
    let tool_spans = records
        .iter()
        .filter_map(|record| match record {
            CapturedTelemetryRecord::SpanStarted(span)
                if span.name.starts_with("execute_tool ") =>
            {
                Some(span)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(tool_spans.len(), 3);
    assert_eq!(tool_spans[0].fidelity, ExecutionFidelity::Opaque);
    assert!(records.iter().any(|record| matches!(
        record,
        CapturedTelemetryRecord::SpanFinished {
            status: ExecutionStatus::Failed,
            error_classification: Some(classification),
            ..
        } if classification == "provider_tool_failed"
    )));
    assert!(records.iter().any(|record| matches!(
        record,
        CapturedTelemetryRecord::SpanFinished {
            status: ExecutionStatus::Incomplete,
            error_classification: Some(classification),
            ..
        } if classification == "provider_boundary_missing"
    )));
    assert_eq!(
        world.messages.lock().expect("messages")[&message.id]
            .tool_use
            .len(),
        4
    );
}

#[test]
fn streaming_tokens_are_coalesced_and_flushed_on_completion() {
    let world = test_world();
    let service = service(world.clone());
    let message = service
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "hello".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("send");
    let sink = world
        .generation_sinks
        .lock()
        .expect("generation sinks")
        .get("process-1")
        .cloned()
        .expect("sink");
    let persisted_content = || {
        world.messages.lock().expect("messages")[&message.id]
            .content
            .clone()
    };

    sink.handle(GenerationProcessEvent::Token("alpha".to_string()))
        .expect("token");
    sink.handle(GenerationProcessEvent::Token("beta".to_string()))
        .expect("token");

    // Both small deltas arrive within the flush window, so persistence is coalesced
    // rather than one full-content rewrite per token (the O(N²) path we removed).
    assert!(
        persisted_content().len() < "alpha\nbeta".len(),
        "streaming deltas must not be persisted per token, got {:?}",
        persisted_content()
    );

    sink.handle(GenerationProcessEvent::Completed)
        .expect("completed");

    // The terminal transition flushes the coalesced tail and the full content is durable.
    assert_eq!(persisted_content(), "alpha\nbeta");
}

#[test]
fn stream_events_persist_complete_usage_and_operation_once() {
    let world = test_world();
    let service = service(world.clone());
    let message = service
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "hello".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("send");
    let sink = world
        .generation_sinks
        .lock()
        .expect("generation sinks")
        .get("process-1")
        .cloned()
        .expect("sink");

    sink.handle(GenerationProcessEvent::RuntimeSessionId(
        "provider-session".to_string(),
    ))
    .expect("session id");
    sink.handle(GenerationProcessEvent::Token("first".to_string()))
        .expect("first token");
    sink.handle(GenerationProcessEvent::Token("second".to_string()))
        .expect("second token");
    sink.handle(GenerationProcessEvent::Thinking("plan".to_string()))
        .expect("thinking");
    sink.handle(GenerationProcessEvent::ToolUse(ToolUseBlock {
        id: "tool-1".to_string(),
        name: "read".to_string(),
        input: Some(serde_json::json!({"path":"README.md"})),
        output: None,
        status: "running".to_string(),
    }))
    .expect("tool");
    sink.handle(GenerationProcessEvent::RichBlock(
        serde_json::json!({"id":"card-1","kind":"card","v":1}),
    ))
    .expect("rich block");
    sink.handle(GenerationProcessEvent::Stderr(
        "provider warning".to_string(),
    ))
    .expect("stderr");
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let first_sink = sink.clone();
    let first_barrier = barrier.clone();
    let first = std::thread::spawn(move || {
        first_barrier.wait();
        first_sink.handle(GenerationProcessEvent::Completed)
    });
    let second_sink = sink.clone();
    let second_barrier = barrier.clone();
    let second = std::thread::spawn(move || {
        second_barrier.wait();
        second_sink.handle(GenerationProcessEvent::Completed)
    });
    barrier.wait();
    first
        .join()
        .expect("first terminal thread")
        .expect("first terminal");
    second
        .join()
        .expect("second terminal thread")
        .expect("second terminal");
    sink.handle(GenerationProcessEvent::Failed(
        GenerationProcessFailure::retryable("late failure must be ignored"),
    ))
    .expect("late terminal");

    let completed = world
        .messages
        .lock()
        .expect("messages")
        .get(&message.id)
        .cloned()
        .expect("completed message");
    assert_eq!(completed.status, "completed");
    assert_eq!(completed.content, "first\nsecond");
    assert_eq!(completed.thinking_content.as_deref(), Some("plan"));
    assert_eq!(completed.tool_use.len(), 1);
    assert_eq!(completed.rich_blocks.len(), 1);
    assert_eq!(
        completed.token_usage,
        Some(MessageTokenUsage {
            input: "effective::hello\nfiles=0".chars().count() as i64,
            output: "first\nsecond".chars().count() as i64,
        })
    );
    assert_eq!(
        world.sessions.lock().expect("sessions")["session-1"]
            .runtime_session_id
            .as_deref(),
        Some("provider-session")
    );
    assert!(world
        .active_generation
        .lock()
        .expect("active generation")
        .is_none());
    assert!(world
        .operations
        .lock()
        .expect("operations")
        .contains(&OperationEvent::Completed(
            "generation-operation-1".to_string()
        )));
    assert_eq!(
        world
            .operations
            .lock()
            .expect("operations")
            .iter()
            .filter(|event| matches!(event, OperationEvent::Completed(_)))
            .count(),
        1
    );
    assert_eq!(
        world
            .logs
            .lock()
            .expect("logs")
            .last()
            .unwrap()
            .operation_id
            .as_deref(),
        Some("generation-operation-1")
    );
}

#[test]
fn loop_role_generation_delivers_one_terminal_completion_and_cancellation_wins_races() {
    for cancelled in [false, true] {
        let world = test_world();
        world
            .sessions
            .lock()
            .expect("sessions")
            .get_mut("session-1")
            .expect("session")
            .loop_ownership = Some(LoopRoleGenerationOwnership {
            run_id: "run-1".to_string(),
            iteration_id: "iteration-1".to_string(),
            role: "worker".to_string(),
        });
        let service = service(world.clone());
        let message = service
            .send_message(SendMessageRequest {
                source: AgentMessageSource::Desktop,
                session_id: "session-1".to_string(),
                content: "implement".to_string(),
                configuration: chat_configuration(),
                file_references: Vec::new(),
            })
            .expect("send");
        let sink = world
            .generation_sinks
            .lock()
            .expect("generation sinks")
            .get("process-1")
            .cloned()
            .expect("sink");

        if cancelled {
            service.stop_generation("session-1").expect("cancel");
            sink.handle(GenerationProcessEvent::Failed(
                GenerationProcessFailure::retryable("late failure"),
            ))
            .expect("late failure ignored");
        } else {
            sink.handle(GenerationProcessEvent::Token("done".to_string()))
                .expect("token");
            sink.handle(GenerationProcessEvent::Completed)
                .expect("complete");
            sink.handle(GenerationProcessEvent::Completed)
                .expect("duplicate complete ignored");
        }

        let terminal = service
            .take_loop_role_completion("session-1")
            .expect("take")
            .expect("terminal");
        assert_eq!(terminal.run_id, "run-1");
        assert_eq!(terminal.iteration_id, "iteration-1");
        assert_eq!(terminal.message_id, message.id);
        assert_eq!(
            terminal.outcome,
            if cancelled {
                LoopRoleGenerationOutcome::Cancelled
            } else {
                LoopRoleGenerationOutcome::Completed
            }
        );
        assert_eq!(terminal.content.as_deref(), (!cancelled).then_some("done"));
        assert_eq!(
            service
                .take_loop_role_completion("session-1")
                .expect("second take"),
            None
        );
    }
}

#[test]
fn stream_failure_uses_safe_message_and_keeps_diagnostic_in_associated_log() {
    let world = test_world();
    let service = service(world.clone());
    let message = service
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "hello".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("send");
    let sink = world
        .generation_sinks
        .lock()
        .expect("generation sinks")
        .get("process-1")
        .cloned()
        .expect("sink");

    sink.handle(GenerationProcessEvent::Failed(
        GenerationProcessFailure::retryable("provider diagnostic secret"),
    ))
    .expect("failed");

    let failed = world
        .messages
        .lock()
        .expect("messages")
        .get(&message.id)
        .cloned()
        .expect("failed message");
    assert_eq!(failed.status, "failed");
    assert_eq!(failed.error.as_deref(), Some("Codex CLI command failed"));
    let log = world
        .logs
        .lock()
        .expect("logs")
        .last()
        .cloned()
        .expect("log");
    assert_eq!(log.message, "provider diagnostic secret");
    assert_eq!(log.operation_id.as_deref(), Some("generation-operation-1"));
    assert!(world
        .operations
        .lock()
        .expect("operations")
        .contains(&OperationEvent::Failed(
            "generation-operation-1".to_string()
        )));
}

#[test]
fn prompt_failure_is_safe_terminal_and_stop_deduplicates_cancelled_events() {
    let failed_world = test_world();
    failed_world.prompt_failure.store(true, Ordering::SeqCst);
    let failed = service(failed_world.clone())
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "hello".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("safe failed message");
    assert_eq!(failed.status, "failed");
    assert_eq!(failed.error.as_deref(), Some("Prompt Hook assembly failed"));
    assert!(failed_world
        .active_generation
        .lock()
        .expect("active generation")
        .is_none());

    let world = test_world();
    let service = service(world.clone());
    let message = service
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "hello".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("send");
    *world.streaming_message_ids.lock().expect("streaming ids") =
        vec![message.id.clone(), "message-3".to_string()];
    let stopped = service.stop_generation("session-1").expect("stop");
    assert!(stopped.process_stopped);
    assert_eq!(
        stopped.cancelled_message_ids,
        vec!["message-2".to_string(), "message-3".to_string()]
    );
    let cancelled = world
        .events
        .lock()
        .expect("events")
        .iter()
        .filter_map(|event| match event {
            AgentEvent::MessageCancelled { message_id, .. } => Some(message_id.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        cancelled,
        BTreeSet::from(["message-2".to_string(), "message-3".to_string()])
    );
    assert_eq!(
        *world.stopped_processes.lock().expect("stopped processes"),
        vec!["process-1".to_string()]
    );
    assert!(world
        .operations
        .lock()
        .expect("operations")
        .contains(&OperationEvent::Cancelled(
            "generation-operation-1".to_string()
        )));
}
