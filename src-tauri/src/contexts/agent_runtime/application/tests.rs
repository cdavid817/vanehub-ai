use super::*;
use crate::contexts::agent_runtime::domain::{
    parse_memory_actions, AgentAvailability, AgentDefinition, AgentDefinitionInput, AgentLifecycle,
    AgentWorkflow, AvailabilityAssessment, InteractionMode, LaunchMetadata, ParsedMemoryActions,
};
use crate::contexts::execution_observability::api::{
    CapturedTelemetryRecord, CapturingExecutionTelemetry, ExecutionEvent, ExecutionFidelity,
    ExecutionRun, ExecutionRunId, ExecutionSettingsPort, ExecutionSpan, ExecutionStatus,
    ExecutionTelemetryError, ExecutionTelemetryPort, ObservabilitySettings,
    RandomExecutionIdentity, SpanId,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod api_agent_management;
mod embedding_models;
mod loop_and_stream_failures;
mod message_dispatch;
mod onepiece_provider;
mod prompt_composition;
mod tool_approval_and_local_profiles;

struct FailingExecutionTelemetry;

impl ExecutionTelemetryPort for FailingExecutionTelemetry {
    fn start_run(&self, _: &ExecutionRun) -> Result<(), ExecutionTelemetryError> {
        Err(ExecutionTelemetryError::Storage("fixture".to_owned()))
    }

    fn finish_run(
        &self,
        _: &ExecutionRunId,
        _: ExecutionStatus,
        _: &str,
        _: Option<&str>,
    ) -> Result<(), ExecutionTelemetryError> {
        Err(ExecutionTelemetryError::Storage("fixture".to_owned()))
    }

    fn start_span(&self, _: &ExecutionSpan) -> Result<(), ExecutionTelemetryError> {
        Err(ExecutionTelemetryError::Storage("fixture".to_owned()))
    }

    fn finish_span(
        &self,
        _: &ExecutionRunId,
        _: &SpanId,
        _: ExecutionStatus,
        _: &str,
        _: Option<&str>,
    ) -> Result<(), ExecutionTelemetryError> {
        Err(ExecutionTelemetryError::Storage("fixture".to_owned()))
    }

    fn record_event(&self, _: &ExecutionEvent) -> Result<(), ExecutionTelemetryError> {
        Err(ExecutionTelemetryError::Storage("fixture".to_owned()))
    }

    fn add_metric(
        &self,
        _: &'static str,
        _: u64,
        _: &[(&'static str, &'static str)],
    ) -> Result<(), ExecutionTelemetryError> {
        Err(ExecutionTelemetryError::Storage("fixture".to_owned()))
    }

    fn shutdown(&self, _: Duration) -> Result<(), ExecutionTelemetryError> {
        Ok(())
    }
}

#[derive(Default)]
struct FakeMessageTerminalCompletions {
    pending: Arc<Mutex<BTreeMap<String, std::sync::mpsc::SyncSender<AgentMessageTerminal>>>>,
}

impl AgentMessageTerminalCompletionPort for FakeMessageTerminalCompletions {
    fn register(
        &self,
        session_id: &str,
    ) -> Result<AgentMessageTerminalReceiver, AgentRuntimeApplicationError> {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        self.pending
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Generation(error.to_string()))?
            .insert(session_id.to_string(), sender);
        let pending = Arc::downgrade(&self.pending);
        let session_id = session_id.to_string();
        Ok(AgentMessageTerminalReceiver::new(
            receiver,
            Box::new(move || {
                if let Some(pending) = pending.upgrade() {
                    if let Ok(mut pending) = pending.lock() {
                        pending.remove(&session_id);
                    }
                }
            }),
        ))
    }

    fn deliver(
        &self,
        terminal: AgentMessageTerminal,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let sender = self
            .pending
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Generation(error.to_string()))?
            .remove(&terminal.session_id);
        Ok(sender.is_some_and(|sender| sender.try_send(terminal).is_ok()))
    }

    fn remove(&self, session_id: &str) -> Result<bool, AgentRuntimeApplicationError> {
        Ok(self
            .pending
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Generation(error.to_string()))?
            .remove(session_id)
            .is_some())
    }
}

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
    Option<PendingPromptExecution>,
);

pub(super) struct FakeWorld {
    agents: Mutex<Vec<AgentDefinition>>,
    expert_roles: Mutex<Vec<crate::contexts::agent_runtime::domain::ExpertRole>>,
    workflow: Mutex<AgentWorkflow>,
    details: Mutex<(String, BTreeMap<String, String>)>,
    pub(super) sessions: Mutex<BTreeMap<String, AgentSession>>,
    pub(super) messages: Mutex<BTreeMap<String, AgentMessage>>,
    pub(super) created_messages: Mutex<Vec<NewAgentMessage>>,
    generation_order: Mutex<Vec<&'static str>>,
    lifecycle_updates: Mutex<Vec<AgentLifecycle>>,
    pub(super) generation_requests: Mutex<Vec<GenerationProcessRequest>>,
    pub(super) generation_sinks: Mutex<BTreeMap<String, Arc<dyn AgentProcessEventSink>>>,
    loop_terminals: Mutex<Vec<LoopRoleGenerationTerminal>>,
    seat_terminals: Mutex<Vec<SeatTurnTerminal>>,
    stopped_processes: Mutex<Vec<String>>,
    launch_failure: AtomicBool,
    prompt_failure: AtomicBool,
    no_prompt_versions: AtomicBool,
    pub(super) events: Mutex<Vec<AgentEvent>>,
    logs: Mutex<Vec<AgentLog>>,
    operations: Mutex<Vec<OperationEvent>>,
    prompt_reports: Mutex<Vec<PromptExecutionReport>>,
    active_generation: Mutex<Option<ActiveGeneration>>,
    streaming_message_ids: Mutex<Vec<String>>,
    next_message_id: AtomicUsize,
    completed_invocation_usage: Mutex<Vec<AgentInvocationUsage>>,
    resolved_approvals: Mutex<Vec<(String, String, ToolApprovalDecision)>>,
    memories: Mutex<Vec<AgentMemory>>,
    /// `add-cli-memory-support` — lets a test simulate `AgentMemoryPort::list_all` failing
    /// without touching `memories` itself, mirroring `personalization_failure`.
    memories_list_failure: AtomicBool,
    /// What `ApiAgentGateway::provider_config` hands back — `None` by default (matching the
    /// shape every pre-existing call site outside this section's own tests relies on), seeded
    /// per test so `update_api_agent`'s "re-validate against the *stored* interface_format"
    /// logic (`add-agent-lifecycle-management` design.md Decision 4) has something to read.
    provider_config: Mutex<Option<ApiProviderConfig>>,
    onepiece_config: Mutex<StoredOnePieceProviderConfig>,
    onepiece_profiles: Mutex<Vec<StoredOnePieceProviderProfile>>,
    endpoint_profile_metadata: Mutex<BTreeMap<String, StoredEndpointProfileMetadata>>,
    hybrid_routing_rules: Mutex<Vec<StoredHybridRoutingRule>>,
    save_onepiece_failure: AtomicBool,
    model_discovery_failure: AtomicBool,
    /// Last request `OnePieceModelDiscoveryPort::list_models` received — lets tests assert which
    /// credential (transient vs. stored) actually reached the discovery call without needing a
    /// real HTTP layer.
    last_model_discovery_request: Mutex<Option<OnePieceModelDiscoveryRequest>>,
    updated_agents: Mutex<Vec<(String, UpdateApiAgentInput)>>,
    delete_api_agent_failure: AtomicBool,
    deleted_agent_ids: Mutex<Vec<String>>,
    stored_credentials: Mutex<Vec<(String, String)>>,
    credential_reads: AtomicUsize,
    current_onepiece_credential: Mutex<Option<String>>,
    profile_credentials: Mutex<BTreeMap<String, String>>,
    removed_credentials: Mutex<Vec<String>>,
    /// Configurable per test (`add-cli-custom-instructions-injection`) — defaults to
    /// `PreGovernanceSettings::safe_fallback()`, matching every pre-existing test's implicit
    /// assumption of "no custom instructions configured".
    personalization_settings: Mutex<PreGovernanceSettings>,
    personalization_failure: AtomicBool,
    /// Every batch of proposals the runtime submitted. Extraction produces these now; it cannot
    /// reach an active memory, so this is where its output is asserted.
    proposals: Mutex<Vec<AgentMemoryProposal>>,
    /// `add-cli-memory-support` — configurable `AgentMemoryExtractionPort::extract` outcome.
    /// Defaults to `Some("Extracted fact.")`, matching every pre-existing test's implicit
    /// assumption of "extraction succeeds and finds something" where it isn't the point under
    /// test. Records every `exchange` argument it was called with for assertions.
    extraction_response: Mutex<Option<String>>,
    extraction_credential_failure: AtomicBool,
    extraction_call_failure: AtomicBool,
    extraction_calls: Mutex<Vec<String>>,
    extraction_manifests: Mutex<Vec<String>>,
}

impl FakeWorld {
    /// Appends a completed message, attributed as `start_generation` writes live rows: seat id only.
    pub(super) fn seed_message(&self, _speaker: &str, seat_index: Option<usize>, content: &str) {
        let mut messages = self.messages.lock().expect("messages");
        let ordinal = messages.len() + 1;
        let id = format!("seeded-{ordinal}");
        messages.insert(
            id.clone(),
            AgentMessage {
                id,
                session_id: "session-1".to_string(),
                speaker_seat_id: seat_index.map(|index| format!("seat-{}", index + 1)),
                seat_index: None,
                role: if seat_index.is_some() {
                    "assistant".to_string()
                } else {
                    "user".to_string()
                },
                content: content.to_string(),
                status: "completed".to_string(),
                tool_use: Vec::new(),
                thinking_content: None,
                rich_blocks: Vec::new(),
                token_usage: None,
                file_references: Vec::new(),
                error: None,
                // Ordered so `recent_messages` returns them in the order they were seeded.
                created_at: format!("2026-08-07T00:00:{ordinal:02}Z"),
                updated_at: format!("2026-08-07T00:00:{ordinal:02}Z"),
                session_sequence: ordinal as u64,
                execution_run_id: None,
            },
        );
    }

    fn new(agents: Vec<AgentDefinition>) -> Self {
        let session = AgentSession {
            id: "session-1".to_string(),
            agent_id: "codex-cli".to_string(),
            seats: Vec::new(),
            interaction_mode: InteractionMode::Cli,
            lifecycle: AgentLifecycle::Idle,
            folder: Some("C:/workspace".to_string()),
            runtime_session_id: None,
            archived: false,
            read_only: false,
            loop_ownership: None,
        };
        Self {
            expert_roles: Mutex::new(Vec::new()),
            agents: Mutex::new(agents),
            workflow: Mutex::new(AgentWorkflow::new("build")),
            details: Mutex::new(("none".to_string(), BTreeMap::new())),
            sessions: Mutex::new(BTreeMap::from([(session.id.clone(), session)])),
            messages: Mutex::new(BTreeMap::new()),
            created_messages: Mutex::new(Vec::new()),
            generation_order: Mutex::new(Vec::new()),
            lifecycle_updates: Mutex::new(Vec::new()),
            generation_requests: Mutex::new(Vec::new()),
            generation_sinks: Mutex::new(BTreeMap::new()),
            loop_terminals: Mutex::new(Vec::new()),
            seat_terminals: Mutex::new(Vec::new()),
            stopped_processes: Mutex::new(Vec::new()),
            launch_failure: AtomicBool::new(false),
            prompt_failure: AtomicBool::new(false),
            no_prompt_versions: AtomicBool::new(false),
            events: Mutex::new(Vec::new()),
            logs: Mutex::new(Vec::new()),
            operations: Mutex::new(Vec::new()),
            prompt_reports: Mutex::new(Vec::new()),
            active_generation: Mutex::new(None),
            streaming_message_ids: Mutex::new(Vec::new()),
            next_message_id: AtomicUsize::new(0),
            completed_invocation_usage: Mutex::new(Vec::new()),
            resolved_approvals: Mutex::new(Vec::new()),
            memories: Mutex::new(Vec::new()),
            memories_list_failure: AtomicBool::new(false),
            provider_config: Mutex::new(None),
            onepiece_config: Mutex::new(StoredOnePieceProviderConfig {
                provider: "VaneHub".to_string(),
                model_id: None,
                interface_format: None,
                base_url: None,
                auto_approve_tools: false,
            }),
            onepiece_profiles: Mutex::new(Vec::new()),
            endpoint_profile_metadata: Mutex::new(BTreeMap::new()),
            hybrid_routing_rules: Mutex::new(Vec::new()),
            save_onepiece_failure: AtomicBool::new(false),
            model_discovery_failure: AtomicBool::new(false),
            last_model_discovery_request: Mutex::new(None),
            updated_agents: Mutex::new(Vec::new()),
            delete_api_agent_failure: AtomicBool::new(false),
            deleted_agent_ids: Mutex::new(Vec::new()),
            stored_credentials: Mutex::new(Vec::new()),
            credential_reads: AtomicUsize::new(0),
            current_onepiece_credential: Mutex::new(None),
            profile_credentials: Mutex::new(BTreeMap::new()),
            removed_credentials: Mutex::new(Vec::new()),
            personalization_settings: Mutex::new(PreGovernanceSettings::safe_fallback()),
            personalization_failure: AtomicBool::new(false),
            proposals: Mutex::new(Vec::new()),
            extraction_response: Mutex::new(Some(
                r#"[{"action":"create","name":"extracted-fact","description":"An extracted fact","body":"Extracted fact."}]"#.to_string(),
            )),
            extraction_credential_failure: AtomicBool::new(false),
            extraction_call_failure: AtomicBool::new(false),
            extraction_calls: Mutex::new(Vec::new()),
            extraction_manifests: Mutex::new(Vec::new()),
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

    fn validate_seat_configuration(
        &self,
        _session: &AgentSession,
        configuration: AgentChatConfiguration,
    ) -> Result<AgentChatConfiguration, AgentRuntimeApplicationError> {
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
            speaker_seat_id: message.speaker_seat_id,
            seat_index: message.seat_index,
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
            session_sequence: 0,
            execution_run_id: None,
        };
        self.messages
            .lock()
            .expect("messages")
            .insert(id, record.clone());
        Ok(record)
    }

    fn start_generation(
        &self,
        request: DurableAgentGenerationStart,
    ) -> Result<DurableAgentGenerationMessages, AgentRuntimeApplicationError> {
        self.generation_order
            .lock()
            .expect("generation order")
            .push("durable-claim");
        self.lifecycle_updates
            .lock()
            .expect("lifecycle updates")
            .push(AgentLifecycle::Starting);
        self.sessions
            .lock()
            .expect("sessions")
            .get_mut(&request.session_id)
            .ok_or_else(|| {
                AgentRuntimeApplicationError::SessionNotFound(request.session_id.clone())
            })?
            .lifecycle = AgentLifecycle::Starting;
        let mut next_sequence = self.messages.lock().expect("messages").len() as u64 + 1;
        let user_message = request
            .user_message
            .map(|message| self.create_message(message))
            .transpose()?
            .map(|mut message| {
                message.session_sequence = next_sequence;
                message.execution_run_id = Some(request.execution_run_id.clone());
                next_sequence += 1;
                self.messages
                    .lock()
                    .expect("messages")
                    .insert(message.id.clone(), message.clone());
                message
            });
        let mut assistant_message = self.create_message(request.assistant_message)?;
        assistant_message.session_sequence = next_sequence;
        assistant_message.execution_run_id = Some(request.execution_run_id);
        self.messages
            .lock()
            .expect("messages")
            .insert(assistant_message.id.clone(), assistant_message.clone());
        Ok(DurableAgentGenerationMessages {
            user_message,
            assistant_message,
        })
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
        if let Some(usage) = completed.invocation_usage {
            self.completed_invocation_usage
                .lock()
                .expect("completed invocation usage")
                .push(usage);
        }
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

    fn update_seat_provider_thread_id(
        &self,
        session_id: &str,
        seat_id: &str,
        provider_thread_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut sessions = self.sessions.lock().expect("sessions");
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| AgentRuntimeApplicationError::SessionNotFound(session_id.to_string()))?;
        if let Some(seat) = session
            .seats
            .iter_mut()
            .find(|seat| seat.seat_id == seat_id)
        {
            seat.provider_thread_id = Some(provider_thread_id.to_string());
        }
        Ok(())
    }

    fn clear_seat_provider_thread_id(
        &self,
        session_id: &str,
        seat_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut sessions = self.sessions.lock().expect("sessions");
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| AgentRuntimeApplicationError::SessionNotFound(session_id.to_string()))?;
        if let Some(seat) = session
            .seats
            .iter_mut()
            .find(|seat| seat.seat_id == seat_id)
        {
            seat.provider_thread_id = None;
        }
        Ok(())
    }

    fn clear_runtime_session_id(
        &self,
        session_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut sessions = self.sessions.lock().expect("sessions");
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| AgentRuntimeApplicationError::SessionNotFound(session_id.to_string()))?;
        session.runtime_session_id = None;
        Ok(())
    }
}

impl AgentCliProfileGateway for FakeWorld {
    fn load(
        &self,
        agent_id: &str,
        _configuration: &AgentChatConfiguration,
        _operation_id: Option<&str>,
    ) -> Result<CliProfileSnapshot, AgentRuntimeApplicationError> {
        Ok(CliProfileSnapshot {
            executable: format!("C:/bin/{agent_id}.exe"),
            global_args: vec!["--model".to_string(), "gpt-5.5".to_string()],
            invocation_args: Vec::new(),
            env: BTreeMap::new(),
        })
    }
}

impl ApiAgentGateway for FakeWorld {
    fn register(
        &self,
        agent_id: &str,
        input: &RegisterApiAgentInput,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        Ok(api_agent(agent_id, &input.display_name, vec!["api"]))
    }

    fn provider_config(
        &self,
        _agent_id: &str,
    ) -> Result<Option<ApiProviderConfig>, AgentRuntimeApplicationError> {
        Ok(self
            .provider_config
            .lock()
            .expect("provider config")
            .clone())
    }

    fn update(
        &self,
        agent_id: &str,
        input: &UpdateApiAgentInput,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        self.updated_agents
            .lock()
            .expect("updated agents")
            .push((agent_id.to_string(), input.clone()));
        Ok(api_agent(agent_id, &input.display_name, vec!["api"]))
    }

    fn delete(&self, agent_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        if self.delete_api_agent_failure.load(Ordering::SeqCst) {
            return Err(AgentRuntimeApplicationError::Validation(
                "Cannot delete this agent: it is still referenced by 1 sessions.".to_string(),
            ));
        }
        self.deleted_agent_ids
            .lock()
            .expect("deleted agent ids")
            .push(agent_id.to_string());
        Ok(())
    }

    fn onepiece_provider_config(
        &self,
    ) -> Result<StoredOnePieceProviderConfig, AgentRuntimeApplicationError> {
        Ok(self
            .onepiece_config
            .lock()
            .expect("onepiece config")
            .clone())
    }

    fn save_onepiece_provider_config(
        &self,
        input: &StoredOnePieceProviderConfig,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        if self.save_onepiece_failure.load(Ordering::SeqCst) {
            return Err(AgentRuntimeApplicationError::Registry(
                "simulated OnePiece persistence failure".to_string(),
            ));
        }
        *self.onepiece_config.lock().expect("onepiece config") = input.clone();
        Ok(api_agent("onepiece", "OnePiece", vec!["api", "native"]))
    }

    fn reset_onepiece_provider_config(
        &self,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        *self.onepiece_config.lock().expect("onepiece config") = StoredOnePieceProviderConfig {
            provider: "VaneHub".to_string(),
            model_id: None,
            interface_format: None,
            base_url: None,
            auto_approve_tools: false,
        };
        self.onepiece_profiles
            .lock()
            .expect("onepiece profiles")
            .clear();
        Ok(api_agent("onepiece", "OnePiece", vec!["api", "native"]))
    }

    fn list_onepiece_provider_profiles(
        &self,
    ) -> Result<Vec<StoredOnePieceProviderProfile>, AgentRuntimeApplicationError> {
        Ok(self
            .onepiece_profiles
            .lock()
            .expect("onepiece profiles")
            .clone())
    }

    fn save_onepiece_provider_profile(
        &self,
        profile: &StoredOnePieceProviderProfile,
    ) -> Result<StoredOnePieceProviderProfile, AgentRuntimeApplicationError> {
        let mut profiles = self.onepiece_profiles.lock().expect("onepiece profiles");
        if profile.active {
            for candidate in profiles.iter_mut() {
                candidate.active = false;
            }
        }
        if let Some(existing) = profiles
            .iter_mut()
            .find(|candidate| candidate.id == profile.id)
        {
            *existing = profile.clone();
        } else {
            profiles.push(profile.clone());
        }
        Ok(profile.clone())
    }

    fn activate_onepiece_provider_profile(
        &self,
        profile_id: &str,
    ) -> Result<StoredOnePieceProviderProfile, AgentRuntimeApplicationError> {
        let mut profiles = self.onepiece_profiles.lock().expect("onepiece profiles");
        if !profiles.iter().any(|profile| profile.id == profile_id) {
            return Err(AgentRuntimeApplicationError::Validation(
                "OnePiece provider profile was not found.".to_string(),
            ));
        }
        for profile in profiles.iter_mut() {
            profile.active = profile.id == profile_id;
        }
        profiles
            .iter()
            .find(|profile| profile.id == profile_id)
            .cloned()
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Validation(
                    "OnePiece provider profile was not found.".to_string(),
                )
            })
    }

    fn delete_onepiece_provider_profile(
        &self,
        profile_id: &str,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let mut profiles = self.onepiece_profiles.lock().expect("onepiece profiles");
        let active = profiles
            .iter()
            .find(|profile| profile.id == profile_id)
            .map(|profile| profile.active)
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Validation(
                    "OnePiece provider profile was not found.".to_string(),
                )
            })?;
        profiles.retain(|profile| profile.id != profile_id);
        self.endpoint_profile_metadata
            .lock()
            .expect("endpoint metadata")
            .remove(profile_id);
        self.hybrid_routing_rules
            .lock()
            .expect("hybrid rules")
            .retain(|rule| {
                rule.preferred_profile_id != profile_id
                    && rule.fallback_profile_id.as_deref() != Some(profile_id)
            });
        Ok(active)
    }

    fn endpoint_profile_metadata(
        &self,
        profile_id: &str,
    ) -> Result<Option<StoredEndpointProfileMetadata>, AgentRuntimeApplicationError> {
        Ok(self
            .endpoint_profile_metadata
            .lock()
            .expect("endpoint metadata")
            .get(profile_id)
            .cloned())
    }

    fn save_endpoint_profile_metadata(
        &self,
        metadata: &StoredEndpointProfileMetadata,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.endpoint_profile_metadata
            .lock()
            .expect("endpoint metadata")
            .insert(metadata.profile_id.clone(), metadata.clone());
        Ok(())
    }

    fn list_hybrid_routing_rules(
        &self,
    ) -> Result<Vec<StoredHybridRoutingRule>, AgentRuntimeApplicationError> {
        Ok(self
            .hybrid_routing_rules
            .lock()
            .expect("hybrid rules")
            .clone())
    }

    fn replace_hybrid_routing_rules(
        &self,
        rules: &[StoredHybridRoutingRule],
    ) -> Result<(), AgentRuntimeApplicationError> {
        *self.hybrid_routing_rules.lock().expect("hybrid rules") = rules.to_vec();
        Ok(())
    }
}

impl ApiCredentialPort for FakeWorld {
    fn store(&self, agent_id: &str, api_key: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.stored_credentials
            .lock()
            .expect("stored credentials")
            .push((agent_id.to_string(), api_key.to_string()));
        if agent_id == "onepiece" {
            *self
                .current_onepiece_credential
                .lock()
                .expect("onepiece credential") = Some(api_key.to_string());
        } else {
            self.profile_credentials
                .lock()
                .expect("profile credentials")
                .insert(agent_id.to_string(), api_key.to_string());
        }
        Ok(())
    }

    fn fetch(&self, agent_id: &str) -> Result<Option<String>, AgentRuntimeApplicationError> {
        self.credential_reads.fetch_add(1, Ordering::SeqCst);
        if agent_id == "onepiece" {
            return Ok(self
                .current_onepiece_credential
                .lock()
                .expect("onepiece credential")
                .clone());
        }
        Ok(self
            .profile_credentials
            .lock()
            .expect("profile credentials")
            .get(agent_id)
            .cloned())
    }

    fn remove(&self, agent_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.removed_credentials
            .lock()
            .expect("removed credentials")
            .push(agent_id.to_string());
        if agent_id == "onepiece" {
            *self
                .current_onepiece_credential
                .lock()
                .expect("onepiece credential") = None;
        } else {
            self.profile_credentials
                .lock()
                .expect("profile credentials")
                .remove(agent_id);
        }
        Ok(())
    }
}

impl ToolApprovalPort for FakeWorld {
    fn resolve(
        &self,
        process_id: &str,
        call_id: &str,
        decision: ToolApprovalDecision,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        self.resolved_approvals
            .lock()
            .expect("resolved approvals")
            .push((process_id.to_string(), call_id.to_string(), decision));
        Ok(true)
    }
}

/// The flat settings shape the runtime used before governance.
///
/// A test fixture now, not a production type: nothing reads settings this way any more. It stays
/// because what these tests are about — whether instructions appear, whether the index appears,
/// whether extraction runs — is stated most clearly as the settings a user had.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PreGovernanceSettings {
    custom_instructions_about_user: String,
    custom_instructions_style_rules: String,
    custom_instructions_enabled: bool,
    memory_enabled: bool,
    memory_tool_assisted_chats_enabled: bool,
    automatic_context_compaction_enabled: bool,
    context_quality_retention_days: i64,
}

impl PreGovernanceSettings {
    fn safe_fallback() -> Self {
        Self {
            custom_instructions_about_user: String::new(),
            custom_instructions_style_rules: String::new(),
            // Enabled with nothing in it: the pre-governance fallback degraded to the behaviour
            // before settings existed, which was an empty block rather than a disabled one.
            custom_instructions_enabled: true,
            memory_enabled: true,
            memory_tool_assisted_chats_enabled: true,
            automatic_context_compaction_enabled: true,
            context_quality_retention_days: 30,
        }
    }

    fn custom_instructions_block(&self) -> Option<String> {
        if !self.custom_instructions_enabled {
            return None;
        }
        let mut parts = Vec::new();
        let style = self.custom_instructions_style_rules.trim();
        let about = self.custom_instructions_about_user.trim();
        if !style.is_empty() {
            parts.push(format!(
                "### Response style
{style}"
            ));
        }
        if !about.is_empty() {
            parts.push(format!(
                "### About the user
{about}"
            ));
        }
        (!parts.is_empty()).then(|| {
            format!(
                "## Custom Instructions
{}",
                parts.join(
                    "

"
                )
            )
        })
    }
}

impl AgentMemoryPort for FakeWorld {
    fn list_all(&self) -> Result<Vec<AgentMemory>, AgentRuntimeApplicationError> {
        if self.memories_list_failure.load(Ordering::SeqCst) {
            return Err(AgentRuntimeApplicationError::Memory(
                "lookup failed".to_string(),
            ));
        }
        Ok(self.memories.lock().expect("memories").clone())
    }

    fn delete(&self, memory_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.memories
            .lock()
            .expect("memories")
            .retain(|memory| memory.id != memory_id);
        Ok(())
    }

    fn delete_all(&self) -> Result<(), AgentRuntimeApplicationError> {
        self.memories.lock().expect("memories").clear();
        Ok(())
    }
}

impl AgentMemoryExtractionPort for FakeWorld {
    fn extract(
        &self,
        exchange: &str,
        existing: &str,
    ) -> Result<ParsedMemoryActions, AgentRuntimeApplicationError> {
        self.extraction_calls
            .lock()
            .expect("extraction calls")
            .push(exchange.to_string());
        self.extraction_manifests
            .lock()
            .expect("extraction manifests")
            .push(existing.to_string());
        if self.extraction_credential_failure.load(Ordering::SeqCst) {
            return Err(AgentRuntimeApplicationError::Credential(
                "no credential".to_string(),
            ));
        }
        if self.extraction_call_failure.load(Ordering::SeqCst) {
            return Err(AgentRuntimeApplicationError::Memory(
                "call failed".to_string(),
            ));
        }
        // Tests still stage a plain string; it is parsed through the real parser here so a fake
        // response that the production path would reject cannot pass silently in a test.
        let staged = self
            .extraction_response
            .lock()
            .expect("extraction response")
            .clone();
        match staged {
            Some(response) => parse_memory_actions(&response).map_err(|error| {
                AgentRuntimeApplicationError::Memory(format!("unusable response: {error}"))
            }),
            None => Ok(ParsedMemoryActions::default()),
        }
    }
}

/// The pre-governance settings a test configured, as the snapshot a runtime now resolves.
///
/// The translation is mechanical and decides nothing. What each test is about — whether
/// instructions appear, whether the index appears, whether extraction runs — is stated in the
/// settings it sets, exactly as it was before governance; only the shape the runtime reads them
/// through has changed.
impl AgentPersonalizationSnapshotPort for FakeWorld {
    fn snapshot(&self, _context: GenerationPersonalizationContext) -> AgentPersonalizationSnapshot {
        if self.personalization_failure.load(Ordering::SeqCst)
            || self.memories_list_failure.load(Ordering::SeqCst)
        {
            return AgentPersonalizationSnapshot::fail_closed("policy_unavailable");
        }
        let settings = self
            .personalization_settings
            .lock()
            .expect("personalization settings")
            .clone();
        let eligible: Vec<AgentMemoryRef> = if settings.memory_enabled {
            self.memories
                .lock()
                .expect("memories")
                .iter()
                .map(|memory| AgentMemoryRef {
                    id: memory.id.clone(),
                    revision: 1,
                    name: memory.name.clone(),
                    description: memory.description.clone(),
                    memory_type: memory.memory_type,
                    updated_at: memory.modified_at,
                })
                .collect()
        } else {
            Vec::new()
        };
        AgentPersonalizationSnapshot {
            revision_token: "fake-world-snapshot".to_string(),
            instruction_block: settings.custom_instructions_block(),
            memory: AgentMemoryAccess {
                read: settings.memory_enabled,
                explicit_save: settings.memory_enabled,
                automatic_extraction: settings.memory_enabled,
                candidate_creation: settings.memory_enabled,
                retrieval_write: settings.memory_enabled,
                delivery: if settings.memory_enabled {
                    AgentMemoryDelivery::IndexOnly
                } else {
                    AgentMemoryDelivery::None
                },
                eligible_total: eligible.len(),
                eligible,
                blocked_reason: (!settings.memory_enabled).then(|| "policy_denied".to_string()),
                automatic_extraction_in_tool_assisted_turns: settings.memory_enabled
                    && settings.memory_tool_assisted_chats_enabled,
            },
            automatic_context_compaction_enabled: settings.automatic_context_compaction_enabled,
            context_quality_retention_days: settings.context_quality_retention_days,
        }
    }

    fn pinned_bodies(
        &self,
        _refs: &[AgentMemoryRef],
    ) -> Result<Vec<AgentMemoryBody>, AgentRuntimeApplicationError> {
        Ok(Vec::new())
    }

    fn propose_memories(
        &self,
        submission: AgentCandidateSubmission,
    ) -> Result<AgentCandidateOutcome, AgentRuntimeApplicationError> {
        let accepted = submission.proposals.len();
        self.proposals
            .lock()
            .expect("proposals")
            .extend(submission.proposals);
        Ok(AgentCandidateOutcome {
            accepted,
            rejected: 0,
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
            trace: if self.no_prompt_versions.load(Ordering::SeqCst) {
                Vec::new()
            } else {
                vec![
                    PromptTrace {
                        hook_id: "system-context".to_string(),
                        status: "applied".to_string(),
                        version: Some(1),
                        content_hash: Some("hash".to_string()),
                        token_estimate: Some(10),
                        reason: None,
                    },
                    PromptTrace {
                        hook_id: "review-focus".to_string(),
                        status: "fired".to_string(),
                        version: Some(2),
                        content_hash: Some("review-hash".to_string()),
                        token_estimate: Some(4),
                        reason: None,
                    },
                ]
            },
        })
    }

    fn record_execution(
        &self,
        report: PromptExecutionReport,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.prompt_reports
            .lock()
            .expect("prompt reports")
            .push(report);
        Ok(())
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
            runner_reference: RunnerReference::local(),
            process_reference: None,
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

impl OnePieceModelDiscoveryPort for FakeWorld {
    fn list_models(
        &self,
        request: OnePieceModelDiscoveryRequest,
    ) -> Result<Vec<OnePieceDiscoveredModel>, AgentRuntimeApplicationError> {
        *self
            .last_model_discovery_request
            .lock()
            .expect("last model discovery request") = Some(request);
        if self.model_discovery_failure.load(Ordering::SeqCst) {
            return Err(AgentRuntimeApplicationError::Validation(
                "simulated model discovery failure".to_string(),
            ));
        }
        Ok(vec![
            OnePieceDiscoveredModel {
                id: "test-chat-model".to_string(),
                display_name: "Test Chat Model".to_string(),
            },
            OnePieceDiscoveredModel {
                id: "test-chat-model".to_string(),
                display_name: "Duplicate".to_string(),
            },
            OnePieceDiscoveredModel {
                id: "test-embedding-model".to_string(),
                display_name: "Embedding".to_string(),
            },
        ])
    }

    fn validate_credential(
        &self,
        _request: ProviderCredentialProbeRequest,
    ) -> Result<ProviderCredentialValidationResult, AgentRuntimeApplicationError> {
        Ok(ProviderCredentialValidationResult {
            status: ProviderCredentialValidationStatus::Valid,
            latency_ms: 1,
            http_status: Some(200),
        })
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
        self.generation_order
            .lock()
            .expect("generation order")
            .push("control-reserve");
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
        *active = Some((lease.clone(), None, None, None, None, None));
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

    fn correlate_prompt(
        &self,
        lease: &GenerationLease,
        execution: &PendingPromptExecution,
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
        current.5 = Some(execution.clone());
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
                |(_, message_id, process_id, operation_id, execution_context, prompt_execution)| {
                    GenerationCancellation {
                        message_id,
                        process_id,
                        operation_id,
                        execution_context,
                        prompt_execution,
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

    fn active_process_id(
        &self,
        _session_id: &str,
    ) -> Result<Option<String>, AgentRuntimeApplicationError> {
        Ok(self
            .active_generation
            .lock()
            .expect("active generation")
            .as_ref()
            .and_then(|current| current.2.clone()))
    }

    fn active_correlation(
        &self,
        _session_id: &str,
    ) -> Result<Option<ActiveGenerationCorrelation>, AgentRuntimeApplicationError> {
        Ok(self
            .active_generation
            .lock()
            .expect("active generation")
            .as_ref()
            .map(|current| ActiveGenerationCorrelation {
                operation_id: current.3.clone(),
                execution_run_id: current
                    .4
                    .as_ref()
                    .map(|context| context.run_id.as_str().to_string()),
            }))
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

fn api_agent(id: &str, display_name: &str, tags: Vec<&str>) -> AgentDefinition {
    AgentDefinition::new(AgentDefinitionInput {
        id: id.to_string(),
        display_name: display_name.to_string(),
        provider: "provider".to_string(),
        managed_sdk_dependency_id: None,
        launch: LaunchMetadata::new("api".to_string(), None, None, None).expect("launch"),
        supported_interaction_modes: vec![InteractionMode::Api],
        availability: AvailabilityAssessment::new(AgentAvailability::Available, None),
        capability_tags: tags.into_iter().map(str::to_string).collect(),
    })
    .expect("agent")
}

fn chat_configuration() -> AgentChatConfiguration {
    AgentChatConfiguration {
        agent_id: "codex-cli".to_string(),
        interaction_mode: InteractionMode::Cli,
        execution_mode: "inherit".to_string(),
        provider_id: Some("openai".to_string()),
        model_id: Some("gpt-5-5".to_string()),
        reasoning_depth: Some("high".to_string()),
        streaming: true,
        thinking: true,
        long_context: false,
    }
}

impl crate::contexts::agent_runtime::application::ExpertRolePort for FakeWorld {
    fn list(
        &self,
    ) -> Result<Vec<crate::contexts::agent_runtime::domain::ExpertRole>, AgentRuntimeApplicationError>
    {
        Ok(self.expert_roles.lock().expect("expert roles").clone())
    }

    fn upsert(
        &self,
        role: &crate::contexts::agent_runtime::domain::ExpertRole,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut roles = self.expert_roles.lock().expect("expert roles");
        roles.retain(|existing| existing.id != role.id);
        roles.push(role.clone());
        Ok(())
    }

    fn delete(&self, role_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.expert_roles
            .lock()
            .expect("expert roles")
            .retain(|role| role.id != role_id);
        Ok(())
    }
}

impl crate::contexts::agent_runtime::application::ConversationHistoryPort for FakeWorld {
    fn recent_messages(
        &self,
        session_id: &str,
        limit: i64,
    ) -> Result<Vec<AgentMessage>, AgentRuntimeApplicationError> {
        let messages = self.messages.lock().expect("messages");
        let mut recent: Vec<AgentMessage> = messages
            .values()
            .filter(|message| message.session_id == session_id)
            .cloned()
            .collect();
        recent.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        let skip = recent.len().saturating_sub(limit.max(0) as usize);
        Ok(recent.split_off(skip))
    }
}

impl SeatTurnCompletionPort for FakeWorld {
    fn deliver(&self, terminal: SeatTurnTerminal) -> Result<bool, AgentRuntimeApplicationError> {
        let mut terminals = self.seat_terminals.lock().expect("seat terminals");
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
    ) -> Result<Option<SeatTurnTerminal>, AgentRuntimeApplicationError> {
        let mut terminals = self.seat_terminals.lock().expect("seat terminals");
        let Some(index) = terminals
            .iter()
            .position(|terminal| terminal.session_id == session_id)
        else {
            return Ok(None);
        };
        Ok(Some(terminals.remove(index)))
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
fn verifier_generation_forces_read_only_execution_mode() {
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
    assert_eq!(requests[0].configuration.execution_mode, "plan");
}

/// A four-seat session, which is enough to exercise the two-mention cap and self-mention
/// filtering at the same time.
pub(super) fn seat_turn_world() -> Arc<FakeWorld> {
    use crate::contexts::agent_runtime::application::AgentSessionSeat;
    use crate::contexts::agent_runtime::domain::{
        ExpertRole, ExpertRoleInput, ExpertRoleReviewPolicy,
    };

    let seats = [
        ("role-architect", "架构师", "claude-code", "Claude Code"),
        ("role-reviewer", "代码审查", "codex-cli", "Codex CLI"),
        ("role-implementer", "实现者", "gemini-cli", "Gemini CLI"),
        ("role-tester", "测试", "opencode", "OpenCode"),
    ];

    let world = Arc::new(FakeWorld::new(
        seats
            .iter()
            .map(|(_, _, agent_id, display_name)| {
                agent(
                    agent_id,
                    display_name,
                    vec![InteractionMode::Cli],
                    vec!["coding"],
                )
            })
            .collect(),
    ));
    {
        let mut roles = world.expert_roles.lock().expect("expert roles");
        for (role_id, role_name, _, _) in seats {
            roles.push(
                ExpertRole::new(
                    role_id.to_string(),
                    ExpertRoleInput {
                        display_name: role_name.to_string(),
                        avatar: "🧭".to_string(),
                        color: "#336699".to_string(),
                        responsibility: format!("{role_name}的职责"),
                        instruction: format!("你是{role_name}。"),
                        skill_ids: Vec::new(),
                        review_policy: ExpertRoleReviewPolicy {
                            peer_reviewer: false,
                            require_different_family: false,
                        },
                        preferred_providers: Vec::new(),
                    },
                    "2026-08-07T00:00:00+00:00".to_string(),
                    "2026-08-07T00:00:00+00:00".to_string(),
                )
                .expect("role"),
            );
        }
    }
    {
        let mut sessions = world.sessions.lock().expect("sessions");
        let session = sessions.get_mut("session-1").expect("session");
        session.agent_id = "claude-code".to_string();
        session.seats = seats
            .iter()
            .enumerate()
            .map(|(index, (role_id, _, agent_id, _))| AgentSessionSeat {
                seat_id: format!("seat-{}", index + 1),
                agent_id: (*agent_id).to_string(),
                role_id: Some((*role_id).to_string()),
                left_at: None,
                provider_thread_id: None,
            })
            .collect();
    }
    world
}

pub(super) fn service(world: Arc<FakeWorld>) -> AgentRuntimeApplicationService {
    service_with_telemetry(world).0
}

pub(super) fn service_with_telemetry(
    world: Arc<FakeWorld>,
) -> (AgentRuntimeApplicationService, CapturingExecutionTelemetry) {
    let telemetry = CapturingExecutionTelemetry::default();
    let service = service_with_telemetry_port(world, Arc::new(telemetry.clone()));
    (service, telemetry)
}

fn service_with_telemetry_port(
    world: Arc<FakeWorld>,
    telemetry: Arc<dyn ExecutionTelemetryPort>,
) -> AgentRuntimeApplicationService {
    AgentRuntimeApplicationService::new(AgentRuntimeApplicationPorts {
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
        telemetry,
        loop_completions: world.clone(),
        seat_completions: world.clone(),
        expert_roles: world.clone(),
        history: world.clone(),
        message_completions: Arc::new(FakeMessageTerminalCompletions::default()),
        api_agents: world.clone(),
        api_credentials: world.clone(),
        onepiece_model_discovery: world.clone(),
        tool_approvals: world.clone(),
        memories: world.clone(),
        memory_extraction: world.clone(),
        runner_discovery: world.clone(),
        personalization: world,
    })
}

impl RunnerDiscoveryPort for FakeWorld {
    fn list(
        &self,
        _session_id: &str,
        _agent_id: &str,
    ) -> Result<Vec<RunnerDescriptor>, AgentRuntimeApplicationError> {
        Ok(vec![RunnerDescriptor {
            selection: RunnerSelection::local(),
            label: "Local".to_string(),
            host_label: Some("This device".to_string()),
            available: true,
            unavailable_reason: None,
            simulated: false,
            capabilities: RunnerCapabilities {
                interactive_input: true,
                pty: false,
                cancellation: true,
                inspection: true,
                recovery: RunnerRecoveryMode::None,
            },
        }])
    }
}

pub(super) fn test_world() -> Arc<FakeWorld> {
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
