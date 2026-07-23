use super::*;
use crate::contexts::agent_runtime::application::{
    AgentMessage, CompleteAgentMessage, EffectivePrompt, GenerationCancellation, GenerationLease,
    GenerationProcessRequest, NewAgentMessage, StartedGenerationProcess, ToolUseBlock,
    WorkflowLaunchOutcome, WorkflowLaunchRequest,
};
use crate::contexts::agent_runtime::domain::{
    AgentAvailability, AgentDefinition, AgentDefinitionInput, AgentLifecycle, AgentWorkflow,
    AvailabilityAssessment, InteractionMode, LaunchMetadata,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

struct TerminalWorld {
    session: Mutex<super::AgentSession>,
    agent_availability: Mutex<AvailabilityAssessment>,
    lifecycle: Mutex<Vec<AgentLifecycle>>,
    terminal: Mutex<Option<AgentTerminalSession>>,
    terminal_requests: Mutex<Vec<AgentTerminalProcessRequest>>,
    fail_terminal: Mutex<bool>,
    logs: Mutex<Vec<AgentLog>>,
    terminal_events: Mutex<Vec<AgentTerminalEvent>>,
    workflow_events: Mutex<usize>,
    stopped: Mutex<Vec<String>>,
}

impl TerminalWorld {
    fn new(session: super::AgentSession) -> Arc<Self> {
        Arc::new(Self {
            session: Mutex::new(session),
            agent_availability: Mutex::new(AvailabilityAssessment::new(
                AgentAvailability::Available,
                None,
            )),
            lifecycle: Mutex::new(Vec::new()),
            terminal: Mutex::new(None),
            terminal_requests: Mutex::new(Vec::new()),
            fail_terminal: Mutex::new(false),
            logs: Mutex::new(Vec::new()),
            terminal_events: Mutex::new(Vec::new()),
            workflow_events: Mutex::new(0),
            stopped: Mutex::new(Vec::new()),
        })
    }

    fn service(self: &Arc<Self>) -> AgentTerminalApplicationService {
        AgentTerminalApplicationService::new(AgentTerminalApplicationPorts {
            registry: self.clone(),
            sessions: self.clone(),
            cli_profiles: self.clone(),
            terminals: self.clone(),
            logging: self.clone(),
            clock: self.clone(),
            events: self.clone(),
            terminal_events: self.clone(),
        })
    }

    fn set_agent_availability(&self, availability: AvailabilityAssessment) {
        *self.agent_availability.lock().expect("agent availability") = availability;
    }
}

impl AgentRegistryRepository for TerminalWorld {
    fn list(&self) -> Result<Vec<AgentDefinition>, AgentRuntimeApplicationError> {
        Ok(vec![agent(
            self.agent_availability
                .lock()
                .expect("agent availability")
                .clone(),
        )])
    }

    fn find(
        &self,
        agent_id: &str,
    ) -> Result<Option<AgentDefinition>, AgentRuntimeApplicationError> {
        Ok((agent_id == "codex-cli").then(|| {
            agent(
                self.agent_availability
                    .lock()
                    .expect("agent availability")
                    .clone(),
            )
        }))
    }
}

impl AgentSessionGateway for TerminalWorld {
    fn find_session(
        &self,
        _session_id: &str,
    ) -> Result<Option<super::AgentSession>, AgentRuntimeApplicationError> {
        Ok(Some(self.session.lock().expect("session").clone()))
    }

    fn validate_configuration(
        &self,
        _session: &super::AgentSession,
        configuration: super::AgentChatConfiguration,
    ) -> Result<super::AgentChatConfiguration, AgentRuntimeApplicationError> {
        Ok(configuration)
    }

    fn compose_prompt(
        &self,
        _session_id: &str,
        _content: &str,
        _file_references: &[super::AgentFileReference],
    ) -> Result<String, AgentRuntimeApplicationError> {
        unused()
    }

    fn create_message(
        &self,
        _message: NewAgentMessage,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        unused()
    }

    fn find_message(
        &self,
        _message_id: &str,
    ) -> Result<Option<AgentMessage>, AgentRuntimeApplicationError> {
        unused()
    }

    fn append_content(
        &self,
        _message_id: &str,
        _content_delta: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unused()
    }

    fn append_thinking(
        &self,
        _message_id: &str,
        _content_delta: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unused()
    }

    fn append_tool_use(
        &self,
        _message_id: &str,
        _tool_use: ToolUseBlock,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unused()
    }

    fn append_rich_block(
        &self,
        _message_id: &str,
        _block: Value,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unused()
    }

    fn complete_message(
        &self,
        _message: CompleteAgentMessage,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        unused()
    }

    fn fail_message(
        &self,
        _message_id: &str,
        _session_id: &str,
        _error: &str,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        unused()
    }

    fn cancel_streaming_messages(
        &self,
        _session_id: &str,
    ) -> Result<Vec<String>, AgentRuntimeApplicationError> {
        unused()
    }

    fn update_lifecycle(
        &self,
        _session_id: &str,
        lifecycle: AgentLifecycle,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.lifecycle.lock().expect("lifecycle").push(lifecycle);
        self.session.lock().expect("session").lifecycle = lifecycle;
        Ok(())
    }

    fn update_runtime_session_id(
        &self,
        _session_id: &str,
        runtime_session_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.session.lock().expect("session").runtime_session_id =
            Some(runtime_session_id.to_string());
        Ok(())
    }
}

impl AgentCliProfileGateway for TerminalWorld {
    fn load(
        &self,
        _agent_id: &str,
        _configuration: &super::AgentChatConfiguration,
    ) -> Result<super::CliProfileSnapshot, AgentRuntimeApplicationError> {
        unused()
    }

    fn load_interactive(
        &self,
        agent_id: &str,
    ) -> Result<super::CliProfileSnapshot, AgentRuntimeApplicationError> {
        Ok(super::CliProfileSnapshot {
            executable: format!("C:/bin/{agent_id}.exe"),
            selections: BTreeMap::new(),
            managed_args: vec!["--strict-config".to_string()],
        })
    }
}

impl AgentTerminalGateway for TerminalWorld {
    fn attach_retained(
        &self,
        _session_id: &str,
    ) -> Result<Option<AgentTerminalSession>, AgentRuntimeApplicationError> {
        Ok(self.terminal.lock().expect("terminal").clone())
    }

    fn open_or_attach(
        &self,
        request: AgentTerminalProcessRequest,
    ) -> Result<AgentTerminalSession, AgentRuntimeApplicationError> {
        if *self.fail_terminal.lock().expect("fail terminal") {
            return Err(AgentRuntimeApplicationError::Process(
                "terminal failed".to_string(),
            ));
        }
        self.terminal_requests
            .lock()
            .expect("terminal requests")
            .push(request.clone());
        let mut terminal = self.terminal.lock().expect("terminal");
        if let Some(existing) = terminal.clone() {
            return Ok(existing);
        }
        let created = AgentTerminalSession {
            terminal_id: "terminal-1".to_string(),
            session_id: request.session.id,
            agent_id: request.agent.id,
            state: AgentTerminalState::Running,
            capability: super::AgentTerminalCapability::Native,
            size: request.size,
            runtime_session_id: Some("runtime-1".to_string()),
            retained: true,
        };
        *terminal = Some(created.clone());
        Ok(created)
    }

    fn input(
        &self,
        _request: AgentTerminalInputRequest,
    ) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }

    fn resize(
        &self,
        _request: ResizeAgentTerminalRequest,
    ) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }

    fn stop(
        &self,
        request: StopAgentTerminalRequest,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        self.stopped
            .lock()
            .expect("stopped")
            .push(request.terminal_id);
        Ok(true)
    }

    fn cleanup_idle(
        &self,
        _idle_after_seconds: i64,
    ) -> Result<Vec<String>, AgentRuntimeApplicationError> {
        Ok(vec!["session-1".to_string()])
    }

    fn shutdown(&self) -> Result<Vec<String>, AgentRuntimeApplicationError> {
        Ok(vec!["session-1".to_string()])
    }
}

impl AgentLoggingPort for TerminalWorld {
    fn record(&self, log: AgentLog) -> Result<(), AgentRuntimeApplicationError> {
        self.logs.lock().expect("logs").push(log);
        Ok(())
    }
}

impl AgentClockPort for TerminalWorld {
    fn now(&self) -> String {
        "2026-07-19T00:00:00Z".to_string()
    }
}

impl AgentEventPort for TerminalWorld {
    fn publish(&self, _event: super::AgentEvent) -> Result<(), AgentRuntimeApplicationError> {
        *self.workflow_events.lock().expect("workflow events") += 1;
        Ok(())
    }
}

impl AgentTerminalEventPort for TerminalWorld {
    fn publish_terminal(
        &self,
        event: AgentTerminalEvent,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.terminal_events
            .lock()
            .expect("terminal events")
            .push(event);
        Ok(())
    }
}

impl super::AgentWorkflowRepository for TerminalWorld {
    fn load(&self) -> Result<AgentWorkflow, AgentRuntimeApplicationError> {
        unused()
    }

    fn save(&self, _workflow: &AgentWorkflow) -> Result<(), AgentRuntimeApplicationError> {
        unused()
    }

    fn load_details(
        &self,
    ) -> Result<(String, BTreeMap<String, String>), AgentRuntimeApplicationError> {
        unused()
    }

    fn save_details(
        &self,
        _adapter: &str,
        _message: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unused()
    }
}

impl super::EffectivePromptGateway for TerminalWorld {
    fn assemble(
        &self,
        _agent_id: &str,
        _session_id: &str,
        _user_prompt: &str,
    ) -> Result<EffectivePrompt, AgentRuntimeApplicationError> {
        unused()
    }
}

impl super::AgentProcessGateway for TerminalWorld {
    fn launch_workflow(
        &self,
        _request: WorkflowLaunchRequest,
    ) -> Result<WorkflowLaunchOutcome, AgentRuntimeApplicationError> {
        unused()
    }

    fn start_generation(
        &self,
        _request: GenerationProcessRequest,
    ) -> Result<StartedGenerationProcess, AgentRuntimeApplicationError> {
        unused()
    }

    fn monitor_generation(
        &self,
        _process_id: &str,
        _sink: Arc<dyn super::AgentProcessEventSink>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unused()
    }

    fn stop_generation(
        &self,
        _process_id: &str,
        _initiator: super::ProcessStopInitiator,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        unused()
    }
}

impl super::AgentTaskPort for TerminalWorld {
    fn start_agent_launch(
        &self,
        _agent_id: &str,
        _message: &str,
    ) -> Result<super::AgentOperation, AgentRuntimeApplicationError> {
        unused()
    }

    fn start_agent_generation(
        &self,
        _agent_id: &str,
        _session_id: &str,
        _message_id: &str,
    ) -> Result<super::AgentOperation, AgentRuntimeApplicationError> {
        unused()
    }

    fn append_log(
        &self,
        _operation_id: &str,
        _line: String,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unused()
    }

    fn complete(&self, _operation_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        unused()
    }

    fn fail(
        &self,
        _operation_id: &str,
        _error: String,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unused()
    }

    fn cancel(&self, _operation_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        unused()
    }
}

impl super::AgentGenerationPort for TerminalWorld {
    fn reserve(&self, _session_id: &str) -> Result<GenerationLease, AgentRuntimeApplicationError> {
        unused()
    }

    fn correlate(
        &self,
        _lease: &GenerationLease,
        _execution_context: &crate::contexts::execution_observability::api::ExecutionContext,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unused()
    }

    fn attach(
        &self,
        _lease: &GenerationLease,
        _message_id: &str,
        _process_id: &str,
        _operation_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unused()
    }

    fn release(&self, _lease: &GenerationLease) -> Result<(), AgentRuntimeApplicationError> {
        unused()
    }

    fn cancel(
        &self,
        _session_id: &str,
    ) -> Result<Option<GenerationCancellation>, AgentRuntimeApplicationError> {
        unused()
    }

    fn complete(&self, _session_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        unused()
    }

    fn fail(&self, _session_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        unused()
    }
}

#[test]
fn open_terminal_starts_session_and_uses_interactive_profile() {
    let world = TerminalWorld::new(session(false));
    let opened = world
        .service()
        .open_or_attach(open_request())
        .expect("open terminal");

    assert_eq!(opened.terminal_id, "terminal-1");
    assert_eq!(
        *world.lifecycle.lock().expect("lifecycle"),
        vec![AgentLifecycle::Starting, AgentLifecycle::Running]
    );
    assert_eq!(
        world.terminal_requests.lock().expect("requests")[0]
            .cli_profile
            .managed_args,
        vec!["--strict-config".to_string()]
    );
    assert_eq!(
        world
            .session
            .lock()
            .expect("session")
            .runtime_session_id
            .as_deref(),
        Some("runtime-1")
    );
    assert_eq!(*world.workflow_events.lock().expect("events"), 1);
}

#[test]
fn stored_runtime_session_id_is_passed_to_terminal_start_for_resume() {
    let mut persisted = session(false);
    persisted.runtime_session_id = Some("provider-resume-1".to_string());
    let world = TerminalWorld::new(persisted);

    world
        .service()
        .open_or_attach(open_request())
        .expect("open terminal");

    assert_eq!(
        world.terminal_requests.lock().expect("requests")[0]
            .session
            .runtime_session_id
            .as_deref(),
        Some("provider-resume-1")
    );
}

#[test]
fn cli_terminal_uses_interactive_profile_even_when_sdk_dependency_is_missing() {
    let world = TerminalWorld::new(session(false));
    world.set_agent_availability(AvailabilityAssessment::new(
        AgentAvailability::Unavailable,
        Some("Managed SDK dependency 'codex-sdk' is not installed.".to_string()),
    ));

    let opened = world
        .service()
        .open_or_attach(open_request())
        .expect("open terminal");

    assert_eq!(opened.terminal_id, "terminal-1");
    assert_eq!(world.terminal_requests.lock().expect("requests").len(), 1);
    assert_eq!(
        world.terminal_requests.lock().expect("requests")[0]
            .cli_profile
            .executable,
        "C:/bin/codex-cli.exe"
    );
}

#[test]
fn archived_session_is_rejected_before_process_start() {
    let world = TerminalWorld::new(session(true));
    let error = world
        .service()
        .open_or_attach(open_request())
        .expect_err("archived rejected");

    assert!(matches!(error, AgentRuntimeApplicationError::Validation(_)));
    assert!(world.terminal_requests.lock().expect("requests").is_empty());
}

#[test]
fn verifier_session_is_rejected_before_terminal_process_start() {
    let mut verifier = session(false);
    verifier.read_only = true;
    let world = TerminalWorld::new(verifier);

    let error = world
        .service()
        .open_or_attach(open_request())
        .expect_err("verifier terminal rejected");

    assert_eq!(
        error,
        AgentRuntimeApplicationError::PolicyDenied {
            session_id: "session-1".to_string(),
            action: "open-terminal".to_string(),
        }
    );
    assert!(world.terminal_requests.lock().expect("requests").is_empty());
    assert!(world
        .logs
        .lock()
        .expect("logs")
        .iter()
        .any(|log| { log.level == AgentLogLevel::Error && log.message.contains("open-terminal") }));
}

#[test]
fn repeated_open_attaches_existing_terminal_without_duplicate_live_state() {
    let world = TerminalWorld::new(session(false));
    let first = world
        .service()
        .open_or_attach(open_request())
        .expect("first");
    let second = world
        .service()
        .open_or_attach(open_request())
        .expect("second");

    assert_eq!(first.terminal_id, second.terminal_id);
    assert_eq!(world.terminal_requests.lock().expect("requests").len(), 1);
    assert_eq!(*world.workflow_events.lock().expect("events"), 2);
}

#[test]
fn terminal_start_failure_marks_session_failed() {
    let world = TerminalWorld::new(session(false));
    *world.fail_terminal.lock().expect("fail flag") = true;

    let error = world
        .service()
        .open_or_attach(open_request())
        .expect_err("terminal failure");

    assert!(matches!(error, AgentRuntimeApplicationError::Process(_)));
    assert_eq!(
        *world.lifecycle.lock().expect("lifecycle"),
        vec![AgentLifecycle::Starting, AgentLifecycle::Failed]
    );
    assert_eq!(
        world.logs.lock().expect("logs").last().unwrap().level,
        AgentLogLevel::Error
    );
    assert_eq!(
        world.logs.lock().expect("logs").last().unwrap().category,
        "session.agent_terminal"
    );
}

#[test]
fn idle_cleanup_and_shutdown_report_stopped_sessions() {
    let world = TerminalWorld::new(session(false));
    let service = world.service();

    assert_eq!(
        service.cleanup_idle(1800).expect("cleanup"),
        vec!["session-1".to_string()]
    );
    assert_eq!(
        service.shutdown().expect("shutdown"),
        vec!["session-1".to_string()]
    );
    assert!(world.logs.lock().expect("logs").len() >= 2);
    assert!(!world
        .terminal_events
        .lock()
        .expect("terminal events")
        .is_empty());
}

fn agent(availability: AvailabilityAssessment) -> AgentDefinition {
    AgentDefinition::new(AgentDefinitionInput {
        id: "codex-cli".to_string(),
        display_name: "Codex CLI".to_string(),
        provider: "OpenAI".to_string(),
        managed_sdk_dependency_id: None,
        launch: LaunchMetadata::new(
            "cli".to_string(),
            Some("codex".to_string()),
            None,
            Some("codex".to_string()),
        )
        .expect("launch"),
        supported_interaction_modes: vec![InteractionMode::Cli],
        availability,
        capability_tags: vec!["coding".to_string()],
    })
    .expect("agent")
}

fn session(archived: bool) -> super::AgentSession {
    super::AgentSession {
        id: "session-1".to_string(),
        agent_id: "codex-cli".to_string(),
        interaction_mode: InteractionMode::Cli,
        lifecycle: AgentLifecycle::Idle,
        folder: Some("D:/work/demo".to_string()),
        runtime_session_id: None,
        archived,
        read_only: false,
        loop_ownership: None,
    }
}

fn open_request() -> OpenAgentTerminalRequest {
    OpenAgentTerminalRequest {
        session_id: "session-1".to_string(),
        size: super::AgentTerminalSize { rows: 24, cols: 80 },
    }
}

fn unused<T>() -> Result<T, AgentRuntimeApplicationError> {
    Err(AgentRuntimeApplicationError::Validation(
        "unused test path".to_string(),
    ))
}
