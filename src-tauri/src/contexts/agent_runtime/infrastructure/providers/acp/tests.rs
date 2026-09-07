//! End-to-end conformance of the ACP gateway against an in-process fake agent.
//!
//! The fake is a scripted peer on the far end of a pipe pair: it speaks real frames, sends real
//! requests mid-prompt, and observes what the host replied. Nothing here spawns a vendor CLI,
//! reads a credential, or reaches the network. Modes are selected per test, never through a
//! production argument.

use super::adapter::{
    test_dependencies, AcpAgentProcessAdapter, AcpAgentProcessDependencies, AcpLauncher,
    ProcessLauncher,
};
use super::binding::SqliteExecutionBindingRepository;
use super::connection::test_support::fake_agent;
use super::connection::{AcpConnection, AcpError, AcpLaunchSpec};
use super::environment::child_environment;
use super::session::{initialize, new_session, HostCapabilities, StopReason};
use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentLog, AgentLoggingPort, AgentPermissionPort, AgentProcessEventSink,
    AgentProcessGateway, AgentRuntimeApplicationError, GenerationProcessEvent,
    GenerationProcessRequest, ManagedConnectionCheckRequest, ManagedConnectionControlPort,
    ProcessStopInitiator, ProviderAcpInvocationRequest, RunnerError, RunnerPermissionContext,
    RunnerPermissionPort, RunnerPolicyWitness, ToolApprovalDecision, ToolApprovalPort,
    ToolLifecyclePhase,
};
use crate::contexts::agent_runtime::infrastructure::providers::builtin_cli_provider_registry;
use crate::contexts::permissions::api::{Action, Effect, Resource};
use crate::test_support::TempDirectory;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct FixedPolicy {
    effect: Effect,
    registered: Mutex<Vec<String>>,
}

impl AgentPermissionPort for FixedPolicy {
    fn evaluate(
        &self,
        _agent_id: &str,
        action: Action,
        _resource: Resource,
        _session_id: &str,
        _generation_id: &str,
        _project_key: &str,
    ) -> Effect {
        if action.as_str() == "file.read" {
            Effect::Allow
        } else {
            self.effect
        }
    }

    fn create_pending_approval(
        &self,
        _agent_id: &str,
        _action: Action,
        _resource: Resource,
        _session_id: &str,
        _generation_id: &str,
        call_id: &str,
        _project_key: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.registered
            .lock()
            .expect("lock")
            .push(call_id.to_string());
        Ok(())
    }
}

struct AllowRunner;

impl RunnerPermissionPort for AllowRunner {
    fn authorize(
        &self,
        _context: &RunnerPermissionContext,
    ) -> Result<RunnerPolicyWitness, RunnerError> {
        Ok(RunnerPolicyWitness {
            fingerprint: "test".to_string(),
        })
    }

    fn revalidate(
        &self,
        _context: &RunnerPermissionContext,
        _witness: &RunnerPolicyWitness,
    ) -> Result<(), RunnerError> {
        Ok(())
    }
}

struct RecordingLog(Mutex<Vec<AgentLog>>);

impl AgentLoggingPort for RecordingLog {
    fn record(&self, log: AgentLog) -> Result<(), AgentRuntimeApplicationError> {
        self.0.lock().expect("lock").push(log);
        Ok(())
    }
}

struct FixedClock;

impl AgentClockPort for FixedClock {
    fn now(&self) -> String {
        "2026-09-06T00:00:00Z".to_string()
    }
}

#[derive(Default)]
struct CapturingSink {
    events: Mutex<Vec<GenerationProcessEvent>>,
}

impl AgentProcessEventSink for CapturingSink {
    fn handle(&self, event: GenerationProcessEvent) -> Result<(), AgentRuntimeApplicationError> {
        self.events.lock().expect("lock").push(event);
        Ok(())
    }
}

impl CapturingSink {
    fn events(&self) -> Vec<GenerationProcessEvent> {
        self.events.lock().expect("lock").clone()
    }

    fn wait_for_terminal(&self, timeout: Duration) -> Vec<GenerationProcessEvent> {
        let deadline = Instant::now() + timeout;
        loop {
            let events = self.events();
            if events.iter().any(|event| {
                matches!(
                    event,
                    GenerationProcessEvent::Completed(_) | GenerationProcessEvent::Failed(_)
                )
            }) {
                return events;
            }
            assert!(Instant::now() < deadline, "no terminal event: {events:?}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

/// Which scripted peer a launch produces.
#[derive(Clone, Copy)]
enum FakeMode {
    Normal,
    PermissionBeforeFinal,
    IgnoreCancel,
    AbruptEof,
    UnsupportedVersion,
    NoLoadSession,
    CursorQuestion,
    /// `session/new` answers the ACP `auth_required` code: nobody is signed in.
    AuthRequired,
}

struct FakeLauncher {
    mode: FakeMode,
    launches: AtomicUsize,
    received: Mutex<Vec<Arc<Mutex<Vec<Value>>>>>,
}

impl FakeLauncher {
    fn new(mode: FakeMode) -> Arc<Self> {
        Arc::new(Self {
            mode,
            launches: AtomicUsize::new(0),
            received: Mutex::new(Vec::new()),
        })
    }

    fn all_received(&self) -> Vec<Value> {
        self.received
            .lock()
            .expect("lock")
            .iter()
            .flat_map(|log| log.lock().expect("lock").clone())
            .collect()
    }
}

fn reply(document: &Value, result: Value) -> Value {
    json!({"jsonrpc":"2.0","id":document["id"],"result":result})
}

fn update(session: &str, update: Value) -> Value {
    json!({"jsonrpc":"2.0","method":"session/update","params":{"sessionId":session,"update":update}})
}

impl AcpLauncher for FakeLauncher {
    fn launch(&self, spec: &AcpLaunchSpec) -> Result<Arc<AcpConnection>, AcpError> {
        assert!(spec.args.contains(&"--acp".to_string()) || spec.args.contains(&"acp".to_string()));
        assert!(spec
            .cwd
            .as_deref()
            .is_some_and(|cwd| std::path::Path::new(cwd).is_absolute()));
        self.launches.fetch_add(1, Ordering::SeqCst);
        let mode = self.mode;
        let prompt_ids = Arc::new(Mutex::new(Vec::<Value>::new()));
        let agent = fake_agent(move |document| {
            let method = document["method"].as_str().unwrap_or("");
            match method {
                "initialize" => {
                    let version = if matches!(mode, FakeMode::UnsupportedVersion) {
                        7
                    } else {
                        1
                    };
                    vec![reply(
                        document,
                        json!({
                            "protocolVersion": version,
                            "agentCapabilities": {"loadSession": !matches!(mode, FakeMode::NoLoadSession), "promptCapabilities": {"image": false}},
                            "agentInfo": {"name": "fake-agent", "version": "0.0.1"},
                            "authMethods": [],
                        }),
                    )]
                }
                "session/new" if matches!(mode, FakeMode::AuthRequired) => vec![json!({
                    "jsonrpc": "2.0",
                    "id": document["id"].clone(),
                    "error": {"code": -32000, "message": "Authentication required: sign in first.", "data": {"authMethods": []}},
                })],
                "session/new" => vec![reply(document, json!({"sessionId": "sess_fake"}))],
                "session/load" => vec![
                    update(
                        "sess_fake",
                        json!({"sessionUpdate":"user_message_chunk","content":{"type":"text","text":"old prompt"}}),
                    ),
                    update(
                        "sess_fake",
                        json!({"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"old reply"}}),
                    ),
                    reply(document, Value::Null),
                ],
                "session/prompt" => {
                    prompt_ids
                        .lock()
                        .expect("lock")
                        .push(document["id"].clone());
                    let text = document["params"]["prompt"][0]["text"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    let mut frames = vec![
                        update(
                            "sess_fake",
                            json!({"sessionUpdate":"agent_thought_chunk","content":{"type":"text","text":"thinking"}}),
                        ),
                        update(
                            "sess_fake",
                            json!({"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":format!("echo: {text}")}}),
                        ),
                    ];
                    match mode {
                        FakeMode::PermissionBeforeFinal => {
                            frames.push(update("sess_fake", json!({"sessionUpdate":"tool_call","toolCallId":"call-1","title":"Write a.txt","kind":"edit","status":"pending","locations":[{"path":"/w/a.txt"}]})));
                            frames.push(json!({"jsonrpc":"2.0","id":"perm-1","method":"session/request_permission","params":{"sessionId":"sess_fake","toolCall":{"toolCallId":"call-1","kind":"edit","title":"Write a.txt"},"options":[{"optionId":"allow-always","name":"Always","kind":"allow_always"},{"optionId":"allow-once","name":"Once","kind":"allow_once"},{"optionId":"reject","name":"No","kind":"reject_once"}]}}));
                        }
                        FakeMode::CursorQuestion => {
                            frames.push(json!({"jsonrpc":"2.0","id":"q-1","method":"cursor/ask_question","params":{"toolCallId":"ask-1","questions":[{"id":"which","prompt":"Which?","options":["a","b"]}]}}));
                        }
                        FakeMode::IgnoreCancel => {
                            // Never answers the prompt; the host must escalate.
                        }
                        FakeMode::AbruptEof => {
                            frames.push(update("sess_fake", json!({"sessionUpdate":"tool_call","toolCallId":"call-eof","title":"edit","kind":"edit","status":"in_progress"})));
                            frames.push(json!({"__close__": true}));
                        }
                        _ => frames.push(reply(document, json!({"stopReason": "end_turn"}))),
                    }
                    frames
                }
                _ if document.get("id") == Some(&json!("perm-1")) => {
                    let selected = document["result"]["outcome"]["optionId"].clone();
                    let status = if selected == json!("allow-once") {
                        "completed"
                    } else {
                        "failed"
                    };
                    let prompt_id = prompt_ids
                        .lock()
                        .expect("lock")
                        .last()
                        .cloned()
                        .unwrap_or(Value::Null);
                    vec![
                        update(
                            "sess_fake",
                            json!({"sessionUpdate":"tool_call_update","toolCallId":"call-1","status":status}),
                        ),
                        json!({"jsonrpc":"2.0","id":prompt_id,"result":{"stopReason":"end_turn"}}),
                    ]
                }
                _ if document.get("id") == Some(&json!("q-1")) => {
                    let prompt_id = prompt_ids
                        .lock()
                        .expect("lock")
                        .last()
                        .cloned()
                        .unwrap_or(Value::Null);
                    vec![
                        update(
                            "sess_fake",
                            json!({"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":format!(" answered={}", document["result"]["outcome"])}}),
                        ),
                        json!({"jsonrpc":"2.0","id":prompt_id,"result":{"stopReason":"end_turn"}}),
                    ]
                }
                "session/cancel" => {
                    if matches!(mode, FakeMode::IgnoreCancel) {
                        Vec::new()
                    } else {
                        let prompt_id = prompt_ids
                            .lock()
                            .expect("lock")
                            .last()
                            .cloned()
                            .unwrap_or(Value::Null);
                        vec![
                            json!({"jsonrpc":"2.0","id":prompt_id,"result":{"stopReason":"cancelled"}}),
                        ]
                    }
                }
                _ => Vec::new(),
            }
        });
        self.received
            .lock()
            .expect("lock")
            .push(agent.received.clone());
        Ok(agent.connection)
    }
}

fn adapter(
    mode: FakeMode,
    effect: Effect,
) -> (AcpAgentProcessAdapter, Arc<FakeLauncher>, Arc<FixedPolicy>) {
    let launcher = FakeLauncher::new(mode);
    let policy = Arc::new(FixedPolicy {
        effect,
        registered: Mutex::new(Vec::new()),
    });
    let adapter = AcpAgentProcessAdapter::new(test_dependencies(
        Arc::new(builtin_cli_provider_registry().expect("registry")),
        policy.clone(),
        Arc::new(AllowRunner),
        Arc::new(RecordingLog(Mutex::new(Vec::new()))),
        Arc::new(FixedClock),
        launcher.clone(),
    ));
    (adapter, launcher, policy)
}

/// Like `adapter`, but with the SQLite binding store wired in, so resume decisions are made
/// against a persisted record rather than always "binding-record-missing".
fn adapter_with_store(
    mode: FakeMode,
    effect: Effect,
    database_directory: &std::path::Path,
) -> (AcpAgentProcessAdapter, Arc<FakeLauncher>, Arc<FixedPolicy>) {
    let launcher = FakeLauncher::new(mode);
    let policy = Arc::new(FixedPolicy {
        effect,
        registered: Mutex::new(Vec::new()),
    });
    let database = crate::platform::database::NativeDatabase::new(database_directory.to_path_buf())
        .expect("database");
    let mut dependencies = test_dependencies(
        Arc::new(builtin_cli_provider_registry().expect("registry")),
        policy.clone(),
        Arc::new(AllowRunner),
        Arc::new(RecordingLog(Mutex::new(Vec::new()))),
        Arc::new(FixedClock),
        launcher.clone(),
    );
    dependencies.bindings = Some(SqliteExecutionBindingRepository::new(database));
    let dependencies: AcpAgentProcessDependencies = dependencies;
    (AcpAgentProcessAdapter::new(dependencies), launcher, policy)
}

fn request(
    agent_id: &str,
    workspace: &std::path::Path,
    prompt: &str,
    resume: Option<&str>,
    interactive: bool,
) -> GenerationProcessRequest {
    request_for(
        "session-acp",
        "/fixture/bin/qwen",
        agent_id,
        workspace,
        prompt,
        resume,
        interactive,
    )
}

fn request_for(
    session_id: &str,
    executable: &str,
    agent_id: &str,
    workspace: &std::path::Path,
    prompt: &str,
    resume: Option<&str>,
    interactive: bool,
) -> GenerationProcessRequest {
    use crate::contexts::agent_runtime::application::{
        AgentChatConfiguration, AgentLaunchView, AgentSession, AgentSessionSeat, AgentView,
        CliProfileSnapshot, RunnerSelection,
    };
    use crate::contexts::agent_runtime::domain::{
        AgentAvailability, AgentLifecycle, AgentOrigin, AutomaticCompactionMode, InteractionMode,
    };
    use crate::contexts::execution_observability::api::{
        CapturePolicy, ExecutionContext, ExecutionRunId, SpanId, TraceId,
    };
    GenerationProcessRequest {
        execution_context: ExecutionContext {
            run_id: ExecutionRunId::parse("018f0f17-4d6a-7e20-b41d-66c5271a28d0").expect("run id"),
            trace_id: TraceId::parse("4bf92f3577b34da6a3ce929d0e0e4736").expect("trace id"),
            span_id: SpanId::parse("00f067aa0ba902b7").expect("span id"),
            capture_policy: CapturePolicy::MetadataOnly,
            sampling_per_million: 0,
            mcp_relay_enabled: false,
        },
        session: AgentSession {
            id: session_id.to_string(),
            agent_id: agent_id.to_string(),
            seats: vec![AgentSessionSeat {
                seat_id: "seat-1".to_string(),
                agent_id: agent_id.to_string(),
                role_id: None,
                left_at: None,
                provider_thread_id: resume.map(str::to_string),
            }],
            interaction_mode: InteractionMode::Cli,
            personalization_mode: "standard".to_string(),
            lifecycle: AgentLifecycle::Running,
            folder: Some(workspace.to_string_lossy().to_string()),
            runtime_session_id: resume.map(str::to_string),
            archived: false,
            read_only: false,
            loop_ownership: None,
        },
        agent: AgentView {
            id: agent_id.to_string(),
            display_name: agent_id.to_string(),
            provider: "fixture".to_string(),
            managed_sdk_dependency_id: None,
            launch: AgentLaunchView {
                kind: "cli".to_string(),
                command: Some("qwen".to_string()),
                url: None,
                executable_name: Some("qwen".to_string()),
            },
            supported_interaction_modes: vec![InteractionMode::Cli],
            availability: AgentAvailability::Available,
            unavailable_reason: None,
            capability_tags: Vec::new(),
            origin: AgentOrigin::Builtin,
        },
        message_id: "message-1".to_string(),
        operation_id: format!("operation-{}", prompt.len()),
        configuration: AgentChatConfiguration {
            agent_id: agent_id.to_string(),
            interaction_mode: InteractionMode::Cli,
            execution_mode: "inherit".to_string(),
            provider_id: None,
            model_id: None,
            reasoning_depth: None,
            streaming: true,
            thinking: false,
            long_context: false,
        },
        effective_prompt: prompt.to_string(),
        file_references: Vec::new(),
        automatic_compaction: AutomaticCompactionMode::Automatic,
        role_briefing: None,
        cli_profile: CliProfileSnapshot {
            executable: executable.to_string(),
            global_args: Vec::new(),
            invocation_args: Vec::new(),
            env: Default::default(),
        },
        interactive,
        runner: RunnerSelection::local(),
        endpoint_profile: None,
        resume_thread_id: resume.map(str::to_string),
    }
}

fn tokens(events: &[GenerationProcessEvent]) -> String {
    events
        .iter()
        .filter_map(|event| match event {
            GenerationProcessEvent::Token(text) => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn two_turns_reuse_one_connection_and_complete_on_stop_reason_not_exit() {
    let workspace = TempDirectory::new("acp-e2e-two-turns");
    let (adapter, launcher, _) = adapter(FakeMode::Normal, Effect::Ask);
    assert!(adapter.handles("qwen-code"));
    assert!(!adapter.handles("claude-code"));
    assert!(!adapter.handles("iflow-cli"));

    let first = adapter
        .start_generation(request(
            "qwen-code",
            workspace.path(),
            "第一轮：你好",
            None,
            true,
        ))
        .expect("first turn");
    assert!(first.process_id.starts_with(super::ACP_PROCESS_PREFIX));
    let sink = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&first.process_id, sink.clone())
        .expect("monitor");
    let events = sink.wait_for_terminal(Duration::from_secs(10));
    assert!(
        matches!(events.first(), Some(GenerationProcessEvent::RuntimeSessionId(id)) if id == "sess_fake")
    );
    assert!(events.iter().any(
        |event| matches!(event, GenerationProcessEvent::Thinking(text) if text == "thinking")
    ));
    assert_eq!(tokens(&events), "echo: 第一轮：你好");
    assert!(matches!(
        events.last(),
        Some(GenerationProcessEvent::Completed(None))
    ));

    // Second turn resumes the recorded external session on the same connection.
    let second = adapter
        .start_generation(request(
            "qwen-code",
            workspace.path(),
            "second",
            Some("sess_fake"),
            true,
        ))
        .expect("second turn");
    let sink = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&second.process_id, sink.clone())
        .expect("monitor");
    let events = sink.wait_for_terminal(Duration::from_secs(10));
    assert_eq!(tokens(&events), "echo: second");
    assert_eq!(
        launcher.launches.load(Ordering::SeqCst),
        1,
        "one process serves both turns"
    );
    let methods: Vec<String> = launcher
        .all_received()
        .iter()
        .filter_map(|document| document["method"].as_str().map(str::to_string))
        .collect();
    assert_eq!(
        methods,
        vec![
            "initialize",
            "session/new",
            "session/prompt",
            "session/prompt"
        ]
    );
    assert_eq!(adapter.release_session("session-acp"), 1);
}

#[test]
fn permission_mid_prompt_is_deferred_answered_once_and_never_widened() {
    let workspace = TempDirectory::new("acp-e2e-permission");
    let (adapter, launcher, policy) = adapter(FakeMode::PermissionBeforeFinal, Effect::Ask);
    let started = adapter
        .start_generation(request(
            "kimi-cli",
            workspace.path(),
            "edit please",
            None,
            true,
        ))
        .expect("turn");
    let sink = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&started.process_id, sink.clone())
        .expect("monitor");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let awaiting = sink.events().iter().any(|event| {
            matches!(event, GenerationProcessEvent::ToolLifecycle(tool) if tool.phase == ToolLifecyclePhase::AwaitingApproval && tool.call_id == "call-1")
        });
        if awaiting {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "approval never surfaced: {:?}",
            sink.events()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        !sink
            .events()
            .iter()
            .any(|event| matches!(event, GenerationProcessEvent::Completed(_))),
        "turn must wait for the decision"
    );
    assert_eq!(
        policy.registered.lock().expect("lock").as_slice(),
        ["call-1"]
    );
    // A decision for another session or an unknown call is refused.
    assert!(!adapter
        .resolve(
            &started.process_id,
            "call-unknown",
            ToolApprovalDecision::Approved
        )
        .expect("resolve"));
    assert!(adapter
        .resolve(
            &started.process_id,
            "call-1",
            ToolApprovalDecision::Approved
        )
        .expect("resolve"));
    // Second click: already consumed.
    assert!(!adapter
        .resolve(
            &started.process_id,
            "call-1",
            ToolApprovalDecision::Approved
        )
        .expect("resolve"));
    let events = sink.wait_for_terminal(Duration::from_secs(10));
    assert!(matches!(
        events.last(),
        Some(GenerationProcessEvent::Completed(None))
    ));
    assert!(events.iter().any(|event| matches!(event, GenerationProcessEvent::ToolLifecycle(tool) if tool.phase == ToolLifecyclePhase::Completed && tool.call_id == "call-1")));
    let replies: Vec<Value> = launcher
        .all_received()
        .into_iter()
        .filter(|document| document.get("id") == Some(&json!("perm-1")))
        .collect();
    assert_eq!(replies.len(), 1);
    assert_eq!(
        replies[0]["result"]["outcome"]["optionId"],
        json!("allow-once")
    );
    adapter.release_session("session-acp");
}

#[test]
fn readonly_policy_denies_without_asking_and_the_tool_does_not_run() {
    let workspace = TempDirectory::new("acp-e2e-deny");
    let (adapter, launcher, policy) = adapter(FakeMode::PermissionBeforeFinal, Effect::Deny);
    let started = adapter
        .start_generation(request("qoder-cli", workspace.path(), "edit", None, true))
        .expect("turn");
    let sink = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&started.process_id, sink.clone())
        .expect("monitor");
    let events = sink.wait_for_terminal(Duration::from_secs(10));
    assert!(
        policy.registered.lock().expect("lock").is_empty(),
        "nothing was asked"
    );
    assert!(events.iter().any(|event| matches!(event, GenerationProcessEvent::ToolLifecycle(tool) if tool.phase == ToolLifecyclePhase::Failed && tool.call_id == "call-1")));
    let reply = launcher
        .all_received()
        .into_iter()
        .find(|document| document.get("id") == Some(&json!("perm-1")))
        .expect("permission reply");
    assert_eq!(reply["result"]["outcome"]["optionId"], json!("reject"));
    adapter.release_session("session-acp");
}

#[test]
fn unattended_runs_reject_permission_requests_and_record_intervention() {
    let workspace = TempDirectory::new("acp-e2e-unattended");
    let (adapter, launcher, policy) = adapter(FakeMode::PermissionBeforeFinal, Effect::Ask);
    let started = adapter
        .start_generation(request(
            "copilot-cli",
            workspace.path(),
            "scheduled",
            None,
            false,
        ))
        .expect("turn");
    let sink = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&started.process_id, sink.clone())
        .expect("monitor");
    let events = sink.wait_for_terminal(Duration::from_secs(10));
    assert!(policy.registered.lock().expect("lock").is_empty());
    assert!(events.iter().any(|event| matches!(event, GenerationProcessEvent::RichBlock(block) if block["meta"]["type"] == json!("acp_unattended_rejection") && block["kind"] == json!("card"))));
    let reply = launcher
        .all_received()
        .into_iter()
        .find(|document| document.get("id") == Some(&json!("perm-1")))
        .expect("permission reply");
    assert_eq!(reply["result"]["outcome"]["optionId"], json!("reject"));
    adapter.release_session("session-acp");
}

#[test]
fn cursor_question_blocks_until_answered_and_reply_is_validated() {
    let workspace = TempDirectory::new("acp-e2e-cursor");
    let (adapter, launcher, _) = adapter(FakeMode::CursorQuestion, Effect::Ask);
    let started = adapter
        .start_generation(request(
            "cursor-agent-cli",
            workspace.path(),
            "plan",
            None,
            true,
        ))
        .expect("turn");
    let sink = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&started.process_id, sink.clone())
        .expect("monitor");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if sink.events().iter().any(|event| matches!(event, GenerationProcessEvent::ToolLifecycle(tool) if tool.phase == ToolLifecyclePhase::AwaitingInput && tool.call_id == "ask-1")) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "question never surfaced: {:?}",
            sink.events()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    // An approval is not an answer to a question.
    assert!(adapter
        .resolve(&started.process_id, "ask-1", ToolApprovalDecision::Approved)
        .is_err());
    assert!(adapter
        .resolve(
            &started.process_id,
            "ask-1",
            ToolApprovalDecision::Answered("b".to_string())
        )
        .expect("answer"));
    let events = sink.wait_for_terminal(Duration::from_secs(10));
    assert!(tokens(&events).contains("answered=\"answered\""));
    let reply = launcher
        .all_received()
        .into_iter()
        .find(|document| document.get("id") == Some(&json!("q-1")))
        .expect("question reply");
    assert_eq!(reply["result"]["answers"]["which"], json!(["b"]));
    adapter.release_session("session-acp");
}

#[test]
fn cooperative_cancel_keeps_the_connection_and_forced_cancel_reaps_it() {
    let workspace = TempDirectory::new("acp-e2e-cancel");
    let (adapter, launcher, _) = adapter(FakeMode::PermissionBeforeFinal, Effect::Ask);
    let started = adapter
        .start_generation(request(
            "qwen-code",
            workspace.path(),
            "cancel me",
            None,
            true,
        ))
        .expect("turn");
    let sink = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&started.process_id, sink.clone())
        .expect("monitor");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !sink.events().iter().any(|event| matches!(event, GenerationProcessEvent::ToolLifecycle(tool) if tool.phase == ToolLifecyclePhase::AwaitingApproval)) {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(adapter
        .stop_generation(&started.process_id, ProcessStopInitiator::User)
        .expect("stop"));
    let events = sink.wait_for_terminal(Duration::from_secs(10));
    assert!(
        matches!(events.last(), Some(GenerationProcessEvent::Failed(failure)) if failure.safe_error.as_deref() == Some("cancelled"))
    );
    // The pending approval was answered cancelled, and a late approval from the UI is refused.
    let pending_reply = launcher
        .all_received()
        .into_iter()
        .find(|document| document.get("id") == Some(&json!("perm-1")))
        .expect("cancelled reply");
    assert_eq!(
        pending_reply["result"]["outcome"]["outcome"],
        json!("cancelled")
    );
    assert!(!adapter
        .resolve(
            &started.process_id,
            "call-1",
            ToolApprovalDecision::Approved
        )
        .expect("late"));
    assert!(launcher
        .all_received()
        .iter()
        .any(|document| document["method"] == json!("session/cancel")));
    // The agent answered `cancelled`, so the connection is still reusable for a new turn.
    let next = adapter
        .start_generation(request(
            "qwen-code",
            workspace.path(),
            "again",
            Some("sess_fake"),
            true,
        ))
        .expect("next turn on the same connection");
    assert_eq!(launcher.launches.load(Ordering::SeqCst), 1);
    let _ = adapter.stop_generation(&next.process_id, ProcessStopInitiator::RuntimeCleanup);
    adapter.release_session("session-acp");

    let workspace = TempDirectory::new("acp-e2e-force");
    let (adapter, _, _) = adapter_for(FakeMode::IgnoreCancel);
    let started = adapter
        .start_generation(request("qwen-code", workspace.path(), "ignore", None, true))
        .expect("turn");
    let sink = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&started.process_id, sink.clone())
        .expect("monitor");
    std::thread::sleep(Duration::from_millis(100));
    assert!(adapter
        .stop_generation(&started.process_id, ProcessStopInitiator::User)
        .expect("stop"));
    let events = sink.wait_for_terminal(Duration::from_secs(10));
    assert!(
        matches!(events.last(), Some(GenerationProcessEvent::Failed(failure)) if failure.diagnostic.contains("terminated"))
    );
    // The binding was retired with its connection: the next turn needs a fresh process.
    assert_eq!(adapter.release_session("session-acp"), 0);
}

fn adapter_for(mode: FakeMode) -> (AcpAgentProcessAdapter, Arc<FakeLauncher>, Arc<FixedPolicy>) {
    adapter(mode, Effect::Ask)
}

#[test]
fn eof_after_tool_activity_is_interrupted_with_unknown_effects_and_not_replayed() {
    let workspace = TempDirectory::new("acp-e2e-eof");
    let (adapter, launcher, _) = adapter_for(FakeMode::AbruptEof);
    let started = adapter
        .start_generation(request(
            "codebuddy-code",
            workspace.path(),
            "write",
            None,
            true,
        ))
        .expect("turn");
    let sink = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&started.process_id, sink.clone())
        .expect("monitor");
    let events = sink.wait_for_terminal(Duration::from_secs(10));
    assert!(events.iter().any(|event| matches!(event, GenerationProcessEvent::RichBlock(block) if block["meta"]["type"] == json!("acp_interrupted") && block["meta"]["effectsUnknown"] == json!(true))));
    match events.last() {
        Some(GenerationProcessEvent::Failed(failure)) => {
            assert!(
                failure.diagnostic.contains("effects unknown"),
                "{}",
                failure.diagnostic
            );
            assert_eq!(failure.kind, crate::contexts::agent_runtime::application::GenerationProcessFailureKind::NonRetryable);
        }
        other => panic!("unexpected terminal {other:?}"),
    }
    let prompts = launcher
        .all_received()
        .iter()
        .filter(|document| document["method"] == json!("session/prompt"))
        .count();
    assert_eq!(prompts, 1, "the prompt was not resent");
    // The dead binding was dropped; nothing to release.
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(adapter.release_session("session-acp"), 0);
}

#[test]
fn unsupported_protocol_version_closes_before_any_prompt() {
    let workspace = TempDirectory::new("acp-e2e-version");
    let (adapter, launcher, _) = adapter_for(FakeMode::UnsupportedVersion);
    let error = adapter
        .start_generation(request("qwen-code", workspace.path(), "hi", None, true))
        .expect_err("incompatible");
    assert!(
        error
            .to_string()
            .contains("acp-protocol-version-unsupported"),
        "{error}"
    );
    let methods: Vec<String> = launcher
        .all_received()
        .iter()
        .filter_map(|document| document["method"].as_str().map(str::to_string))
        .collect();
    assert_eq!(methods, vec!["initialize"]);
}

#[test]
fn resume_requires_peer_load_support_and_a_recorded_binding() {
    let workspace = TempDirectory::new("acp-e2e-resume");
    let (adapter, launcher, _) = adapter_for(FakeMode::NoLoadSession);
    // A stored thread id with no persisted binding record cannot be resumed: no guessing.
    let error = adapter
        .start_generation(request(
            "kimi-cli",
            workspace.path(),
            "resume",
            Some("sess_old"),
            true,
        ))
        .expect_err("refused");
    assert!(
        error.to_string().contains("binding-record-missing"),
        "{error}"
    );
    assert!(!launcher
        .all_received()
        .iter()
        .any(|document| document["method"] == json!("session/load")
            || document["method"] == json!("session/prompt")));
    // A fresh session on the same provider still works.
    let started = adapter
        .start_generation(request("kimi-cli", workspace.path(), "fresh", None, true))
        .expect("fresh");
    let sink = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&started.process_id, sink.clone())
        .expect("monitor");
    let events = sink.wait_for_terminal(Duration::from_secs(10));
    assert_eq!(tokens(&events), "echo: fresh");
    adapter.release_session("session-acp");
}

#[test]
fn legacy_and_headless_providers_are_refused_by_the_acp_gateway() {
    let workspace = TempDirectory::new("acp-e2e-refuse");
    let (adapter, launcher, _) = adapter_for(FakeMode::Normal);
    for agent_id in ["iflow-cli", "claude-code"] {
        let error = adapter
            .start_generation(request(agent_id, workspace.path(), "hi", None, true))
            .expect_err(agent_id);
        assert!(
            error.to_string().contains("acp-stdio"),
            "{agent_id}: {error}"
        );
    }
    assert_eq!(launcher.launches.load(Ordering::SeqCst), 0);
    let relative = request(
        "qwen-code",
        std::path::Path::new("relative/path"),
        "hi",
        None,
        true,
    );
    assert!(adapter.start_generation(relative).is_err());
}

#[test]
fn stop_reasons_other_than_end_turn_are_preserved() {
    assert_eq!(StopReason::parse("refusal"), StopReason::Refusal);
    assert_eq!(StopReason::parse("max_tokens").as_str(), "max_tokens");
    assert_eq!(StopReason::parse("weird").as_str(), "weird");
    assert_eq!(StopReason::parse("cancelled"), StopReason::Cancelled);
}

/// Two sessions on the same provider run at the same time on two processes. Neither sees the
/// other's output, and releasing one leaves the other's process alone.
#[test]
fn two_sessions_run_concurrently_on_isolated_processes() {
    let workspace_a = TempDirectory::new("acp-e2e-seat-a");
    let workspace_b = TempDirectory::new("acp-e2e-seat-b");
    let (adapter, launcher, _) = adapter(FakeMode::Normal, Effect::Ask);

    let started_a = adapter
        .start_generation(request_for(
            "session-a",
            "/fixture/bin/qwen",
            "qwen-code",
            workspace_a.path(),
            "alpha",
            None,
            true,
        ))
        .expect("session a");
    let started_b = adapter
        .start_generation(request_for(
            "session-b",
            "/fixture/bin/qwen",
            "qwen-code",
            workspace_b.path(),
            "beta",
            None,
            true,
        ))
        .expect("session b");
    assert_ne!(started_a.process_id, started_b.process_id);
    assert_eq!(
        launcher.launches.load(Ordering::SeqCst),
        2,
        "one process per session"
    );

    let sink_a = Arc::new(CapturingSink::default());
    let sink_b = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&started_a.process_id, sink_a.clone())
        .expect("monitor a");
    adapter
        .monitor_generation(&started_b.process_id, sink_b.clone())
        .expect("monitor b");
    let events_a = sink_a.wait_for_terminal(Duration::from_secs(10));
    let events_b = sink_b.wait_for_terminal(Duration::from_secs(10));
    assert_eq!(tokens(&events_a), "echo: alpha");
    assert_eq!(tokens(&events_b), "echo: beta");
    assert!(matches!(
        events_a.last(),
        Some(GenerationProcessEvent::Completed(None))
    ));
    assert!(matches!(
        events_b.last(),
        Some(GenerationProcessEvent::Completed(None))
    ));

    // Releasing one session touches only its own binding; the other stays pooled.
    assert_eq!(adapter.release_session("session-a"), 1);
    assert_eq!(adapter.release_session("session-a"), 0);
    assert_eq!(adapter.release_session("session-b"), 1);
}

/// The executable changed between turns (an upgrade or a moved installation). The pooled
/// process is retired instead of trusted, and the recorded binding refuses to resume: the reason
/// names the drift, nothing is loaded, and no prompt is replayed.
#[test]
fn installation_drift_between_turns_retires_the_binding_and_refuses_resume() {
    let workspace = TempDirectory::new("acp-e2e-drift");
    let database = TempDirectory::new("acp-e2e-drift-db");
    let (adapter, launcher, _) = adapter_with_store(FakeMode::Normal, Effect::Ask, database.path());

    let first = adapter
        .start_generation(request_for(
            "session-acp",
            "/fixture/bin/qwen",
            "qwen-code",
            workspace.path(),
            "first",
            None,
            true,
        ))
        .expect("first turn");
    let sink = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&first.process_id, sink.clone())
        .expect("monitor");
    let events = sink.wait_for_terminal(Duration::from_secs(10));
    assert_eq!(tokens(&events), "echo: first");

    // Same session, same recorded external id, different program on disk.
    let error = adapter
        .start_generation(request_for(
            "session-acp",
            "/fixture/bin/qwen-upgraded",
            "qwen-code",
            workspace.path(),
            "second",
            Some("sess_fake"),
            true,
        ))
        .expect_err("drift refuses resume");
    assert!(
        error.to_string().contains("binding-installation-changed"),
        "{error}"
    );
    let methods: Vec<String> = launcher
        .all_received()
        .iter()
        .filter_map(|document| document["method"].as_str().map(str::to_string))
        .collect();
    assert!(!methods.iter().any(|method| method == "session/load"));
    assert_eq!(
        methods
            .iter()
            .filter(|method| *method == "session/prompt")
            .count(),
        1,
        "the first prompt was not replayed"
    );

    // The same session can still start a fresh external session on the new program.
    let fresh = adapter
        .start_generation(request_for(
            "session-acp",
            "/fixture/bin/qwen-upgraded",
            "qwen-code",
            workspace.path(),
            "fresh",
            None,
            true,
        ))
        .expect("fresh turn on the new program");
    let sink = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&fresh.process_id, sink.clone())
        .expect("monitor");
    let events = sink.wait_for_terminal(Duration::from_secs(10));
    assert_eq!(tokens(&events), "echo: fresh");
    adapter.release_session("session-acp");
}

/// The explicit connection check is a handshake and nothing else: `initialize`, then the process
/// is released. No session is created, no prompt is sent, and nothing stays pooled.
#[test]
fn connection_check_handshakes_only_and_releases_the_process() {
    let workspace = TempDirectory::new("acp-check");
    let (adapter, launcher, _) = adapter(FakeMode::Normal, Effect::Ask);
    let report = adapter
        .check_connection(ManagedConnectionCheckRequest {
            agent_id: "qwen-code".to_string(),
            executable: "/fixture/bin/qwen".to_string(),
            workspace: workspace.path().to_string_lossy().to_string(),
            provider_id: None,
        })
        .expect("check");
    assert_eq!(report.transport, "acp-stdio");
    assert_eq!(report.protocol_version, 1);
    assert!(report.load_session);
    assert_eq!(report.agent_name.as_deref(), Some("fake-agent"));
    assert_eq!(report.agent_version.as_deref(), Some("0.0.1"));
    let methods: Vec<String> = launcher
        .all_received()
        .iter()
        .filter_map(|document| document["method"].as_str().map(str::to_string))
        .collect();
    assert_eq!(methods, vec!["initialize"]);
    assert_eq!(launcher.launches.load(Ordering::SeqCst), 1);
    // Nothing was bound: a later release finds no connection to drop.
    assert_eq!(adapter.release_session("cli-management"), 0);

    // A tool without an ACP transport is refused before any process starts.
    let error = adapter
        .check_connection(ManagedConnectionCheckRequest {
            agent_id: "iflow-cli".to_string(),
            executable: "/fixture/bin/iflow".to_string(),
            workspace: workspace.path().to_string_lossy().to_string(),
            provider_id: None,
        })
        .expect_err("legacy refused");
    assert!(error.contains("no managed ACP transport"), "{error}");
    assert_eq!(launcher.launches.load(Ordering::SeqCst), 1);
}

#[test]
fn connection_check_reports_an_unsupported_protocol_as_a_failure() {
    let workspace = TempDirectory::new("acp-check-version");
    let (adapter, launcher, _) = adapter_for(FakeMode::UnsupportedVersion);
    let error = adapter
        .check_connection(ManagedConnectionCheckRequest {
            agent_id: "kimi-cli".to_string(),
            executable: "/fixture/bin/kimi".to_string(),
            workspace: workspace.path().to_string_lossy().to_string(),
            provider_id: None,
        })
        .expect_err("version mismatch");
    assert!(
        error.contains("acp-protocol-version-unsupported"),
        "{error}"
    );
    assert!(!launcher
        .all_received()
        .iter()
        .any(|document| document["method"] == json!("session/new")));
}

/// An agent that answers `session/new` with ACP's `auth_required` code is reported as "not
/// signed in", not as a protocol violation, and nothing is prompted or replayed.
#[test]
fn an_unauthenticated_agent_is_reported_as_sign_in_required() {
    let workspace = TempDirectory::new("acp-auth-required");
    let (adapter, launcher, _) = adapter_for(FakeMode::AuthRequired);
    let error = adapter
        .start_generation(request("qwen-code", workspace.path(), "hello", None, true))
        .expect_err("refused");
    let message = error.to_string();
    assert!(message.contains("is not signed in"), "{message}");
    assert!(
        message.contains("Sign in with the CLI in a terminal"),
        "{message}"
    );
    assert!(!message.contains("protocol violation"), "{message}");
    assert!(!launcher
        .all_received()
        .iter()
        .any(|document| document["method"] == json!("session/prompt")));
    assert_eq!(
        adapter.release_session("session-acp"),
        0,
        "nothing stayed pooled"
    );
}

/// Live ACP handshake against the six agents installed on this host. Opt-in through
/// `VANEHUB_LIVE_CLI=1`. Each agent is started with its reviewed ACP flag, sent `initialize`
/// only, and terminated: no session, no prompt, no sign-in, no model call. One `LIVE-ACP` line
/// per agent is the evidence; an agent that refuses the handshake is reported, not hidden.
#[test]
fn live_handshake_with_the_installed_acp_agents() {
    if std::env::var("VANEHUB_LIVE_CLI").ok().as_deref() != Some("1") {
        eprintln!("live handshake skipped: VANEHUB_LIVE_CLI is not set");
        return;
    }
    let workspace = TempDirectory::new("acp-live-handshake");
    let policy = Arc::new(FixedPolicy {
        effect: Effect::Ask,
        registered: Mutex::new(Vec::new()),
    });
    let adapter = AcpAgentProcessAdapter::new(test_dependencies(
        Arc::new(builtin_cli_provider_registry().expect("registry")),
        policy,
        Arc::new(AllowRunner),
        Arc::new(RecordingLog(Mutex::new(Vec::new()))),
        Arc::new(FixedClock),
        Arc::new(ProcessLauncher),
    ));
    let path = std::env::var_os("PATH").unwrap_or_default();
    let resolve = |name: &str| {
        std::env::split_paths(&path)
            .map(|dir| dir.join(name))
            .find(|candidate| candidate.is_file())
            .map(|candidate| candidate.to_string_lossy().to_string())
    };
    let mut successes = 0;
    for (agent_id, executable) in [
        ("qwen-code", "qwen"),
        ("kimi-cli", "kimi"),
        ("qoder-cli", "qoder"),
        ("codebuddy-code", "codebuddy"),
        ("copilot-cli", "copilot"),
        ("cursor-agent-cli", "agent"),
    ] {
        let Some(executable) = resolve(executable) else {
            eprintln!("LIVE-ACP {agent_id}: NOT INSTALLED ({executable} not on PATH)");
            continue;
        };
        let started = Instant::now();
        match adapter.check_connection(ManagedConnectionCheckRequest {
            agent_id: agent_id.to_string(),
            executable: executable.clone(),
            workspace: workspace.path().to_string_lossy().to_string(),
            provider_id: None,
        }) {
            Ok(report) => {
                successes += 1;
                eprintln!(
                    "LIVE-ACP {agent_id}: OK executable={executable} protocol={} loadSession={} agent={:?} version={:?} authMethods={:?} elapsedMs={}",
                    report.protocol_version,
                    report.load_session,
                    report.agent_name,
                    report.agent_version,
                    report.auth_methods,
                    report.elapsed_ms
                );
                assert_eq!(report.protocol_version, 1, "{agent_id}");
            }
            Err(error) => eprintln!(
                "LIVE-ACP {agent_id}: FAILED executable={executable} after {}ms: {error}",
                started.elapsed().as_millis()
            ),
        }
    }
    assert!(
        successes > 0,
        "no installed agent completed an ACP handshake"
    );
}

/// Live negative path for a missing sign-in: after a real `initialize`, ask each installed agent
/// for `session/new` without ever authenticating. What the agent answers -- an auth-required
/// error, or a session id it will later refuse to prompt -- is recorded; no prompt is sent, so no
/// model is called and no login is started. Opt-in through `VANEHUB_LIVE_CLI=1`.
#[test]
fn live_session_new_without_sign_in_is_classified_not_guessed() {
    if std::env::var("VANEHUB_LIVE_CLI").ok().as_deref() != Some("1") {
        eprintln!("live session probe skipped: VANEHUB_LIVE_CLI is not set");
        return;
    }
    let workspace = TempDirectory::new("acp-live-session");
    let registry = builtin_cli_provider_registry().expect("registry");
    let path = std::env::var_os("PATH").unwrap_or_default();
    let resolve = |name: &str| {
        std::env::split_paths(&path)
            .map(|dir| dir.join(name))
            .find(|candidate| candidate.is_file())
            .map(|candidate| candidate.to_string_lossy().to_string())
    };
    let mut probed = 0;
    for (agent_id, executable) in [
        ("qwen-code", "qwen"),
        ("kimi-cli", "kimi"),
        ("qoder-cli", "qoder"),
        ("codebuddy-code", "codebuddy"),
        ("copilot-cli", "copilot"),
        ("cursor-agent-cli", "agent"),
    ] {
        let Some(executable) = resolve(executable) else {
            eprintln!("LIVE-SESSION {agent_id}: NOT INSTALLED");
            continue;
        };
        let provider = registry.get(agent_id).expect("provider");
        let spec = provider
            .prepare_acp(ProviderAcpInvocationRequest {
                executable: executable.clone(),
                global_args: &[],
                invocation_args: &[],
                account_profile: None,
            })
            .expect("acp spec");
        let launch = AcpLaunchSpec {
            executable: spec.executable.clone(),
            args: spec.args.clone(),
            environment: child_environment(agent_id, std::env::vars(), &spec.environment),
            cwd: Some(workspace.path().to_string_lossy().to_string()),
        };
        let connection = match ProcessLauncher.launch(&launch) {
            Ok(connection) => connection,
            Err(error) => {
                eprintln!("LIVE-SESSION {agent_id}: LAUNCH FAILED {error}");
                continue;
            }
        };
        let negotiated = initialize(
            &connection,
            HostCapabilities {
                fs: true,
                terminal: true,
            },
        );
        let outcome = match negotiated {
            Ok(_) => match new_session(&connection, &workspace.path().to_string_lossy()) {
                Ok(session_id) => format!(
                    "session/new ACCEPTED without sign-in (sessionId len {}); prompt deliberately not sent",
                    session_id.len()
                ),
                Err(error) => {
                    // Every refusal in the live evidence is a missing sign-in, and it must be
                    // reported as such rather than as a protocol violation.
                    assert_eq!(error.reason_code(), "acp-authentication-required", "{agent_id}: {error}");
                    format!("session/new REFUSED: {} ({error})", error.reason_code())
                }
            },
            Err(error) => format!("initialize failed: {error}"),
        };
        let stderr = connection.stderr_summary();
        let _ = connection.terminate("acp-live-probe-complete", "probe finished");
        eprintln!("LIVE-SESSION {agent_id}: {outcome}; stderr={stderr:?}");
        probed += 1;
    }
    assert!(probed > 0);
}

/// Live positive path: one real prompt turn through the production adapter (`ProcessLauncher`,
/// real permission policy set to deny, real `child_environment`). The agent's own credentials
/// come from the parent environment and pass through `child_environment` unchanged, so a
/// third-party OpenAI-compatible endpoint (for example DeepSeek behind `OPENAI_BASE_URL`) can be
/// exercised without writing the CLI's global configuration. Opt-in through `VANEHUB_LIVE_CLI=1`
/// plus `VANEHUB_LIVE_PROMPT_AGENT=<agent id>`; `VANEHUB_LIVE_PROMPT_ARGS` (whitespace separated)
/// is appended to the launch as the profile's global arguments. This is a paid model call.
#[test]
fn live_prompt_turn_completes_through_the_acp_adapter() {
    if std::env::var("VANEHUB_LIVE_CLI").ok().as_deref() != Some("1") {
        eprintln!("live prompt skipped: VANEHUB_LIVE_CLI is not set");
        return;
    }
    let Ok(agent_id) = std::env::var("VANEHUB_LIVE_PROMPT_AGENT") else {
        eprintln!("live prompt skipped: VANEHUB_LIVE_PROMPT_AGENT is not set");
        return;
    };
    let executable_name = match agent_id.as_str() {
        "qwen-code" => "qwen",
        "kimi-cli" => "kimi",
        "qoder-cli" => "qoder",
        "codebuddy-code" => "codebuddy",
        "copilot-cli" => "copilot",
        "cursor-agent-cli" => "agent",
        other => panic!("{other} is not an ACP-capable built-in agent"),
    };
    let path = std::env::var_os("PATH").unwrap_or_default();
    let Some(executable) = std::env::split_paths(&path)
        .map(|dir| dir.join(executable_name))
        .find(|candidate| candidate.is_file())
        .map(|candidate| candidate.to_string_lossy().to_string())
    else {
        eprintln!("LIVE-PROMPT {agent_id}: NOT INSTALLED");
        return;
    };

    let workspace = TempDirectory::new("acp-live-prompt");
    let policy = Arc::new(FixedPolicy {
        effect: Effect::Deny,
        registered: Mutex::new(Vec::new()),
    });
    let adapter = AcpAgentProcessAdapter::new(test_dependencies(
        Arc::new(builtin_cli_provider_registry().expect("registry")),
        policy,
        Arc::new(AllowRunner),
        Arc::new(RecordingLog(Mutex::new(Vec::new()))),
        Arc::new(FixedClock),
        Arc::new(ProcessLauncher),
    ));
    let mut request = request_for(
        "session-live-prompt",
        &executable,
        &agent_id,
        workspace.path(),
        "Reply with exactly the word OK and nothing else.",
        None,
        true,
    );
    request.cli_profile.global_args = std::env::var("VANEHUB_LIVE_PROMPT_ARGS")
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect();

    let started = Instant::now();
    let turn = adapter.start_generation(request).expect("start live turn");
    let sink = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&turn.process_id, sink.clone())
        .expect("monitor");
    let events = sink.wait_for_terminal(Duration::from_secs(180));
    let text = tokens(&events);
    eprintln!(
        "LIVE-PROMPT {agent_id}: {} events in {:?}; tokens={text:?}; last={:?}",
        events.len(),
        started.elapsed(),
        events.last()
    );
    assert!(
        matches!(events.last(), Some(GenerationProcessEvent::Completed(_))),
        "the live turn did not complete: {events:?}"
    );
    assert!(
        text.to_uppercase().contains("OK"),
        "unexpected live reply: {text:?}"
    );
    assert_eq!(adapter.release_session("session-live-prompt"), 1);
}

/// The installed agent a live test drives: `(agent id, executable path)`, or `None` with the
/// reason printed when the opt-in variables or the program are missing.
fn live_prompt_target() -> Option<(String, String)> {
    if std::env::var("VANEHUB_LIVE_CLI").ok().as_deref() != Some("1") {
        eprintln!("live prompt skipped: VANEHUB_LIVE_CLI is not set");
        return None;
    }
    let Ok(agent_id) = std::env::var("VANEHUB_LIVE_PROMPT_AGENT") else {
        eprintln!("live prompt skipped: VANEHUB_LIVE_PROMPT_AGENT is not set");
        return None;
    };
    let executable_name = match agent_id.as_str() {
        "qwen-code" => "qwen",
        "kimi-cli" => "kimi",
        "qoder-cli" => "qoder",
        "codebuddy-code" => "codebuddy",
        "copilot-cli" => "copilot",
        "cursor-agent-cli" => "agent",
        other => panic!("{other} is not an ACP-capable built-in agent"),
    };
    let path = std::env::var_os("PATH").unwrap_or_default();
    let executable = std::env::split_paths(&path)
        .map(|dir| dir.join(executable_name))
        .find(|candidate| candidate.is_file())
        .map(|candidate| candidate.to_string_lossy().to_string());
    if executable.is_none() {
        eprintln!("LIVE-PROMPT {agent_id}: NOT INSTALLED");
    }
    executable.map(|executable| (agent_id, executable))
}

/// Production launcher and environment, fixed policy effect, optional persisted binding store.
fn live_adapter(
    effect: Effect,
    database_directory: Option<&std::path::Path>,
) -> (AcpAgentProcessAdapter, Arc<FixedPolicy>) {
    let policy = Arc::new(FixedPolicy {
        effect,
        registered: Mutex::new(Vec::new()),
    });
    let mut dependencies = test_dependencies(
        Arc::new(builtin_cli_provider_registry().expect("registry")),
        policy.clone(),
        Arc::new(AllowRunner),
        Arc::new(RecordingLog(Mutex::new(Vec::new()))),
        Arc::new(FixedClock),
        Arc::new(ProcessLauncher),
    );
    if let Some(directory) = database_directory {
        let database = crate::platform::database::NativeDatabase::new(directory.to_path_buf())
            .expect("database");
        dependencies.bindings = Some(SqliteExecutionBindingRepository::new(database));
    }
    (AcpAgentProcessAdapter::new(dependencies), policy)
}

fn live_request(
    session_id: &str,
    target: &(String, String),
    workspace: &std::path::Path,
    prompt: &str,
    resume: Option<&str>,
) -> GenerationProcessRequest {
    let mut request = request_for(
        session_id, &target.1, &target.0, workspace, prompt, resume, true,
    );
    request.cli_profile.global_args = std::env::var("VANEHUB_LIVE_PROMPT_ARGS")
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect();
    request
}

fn wait_until(
    sink: &CapturingSink,
    timeout: Duration,
    what: &str,
    predicate: impl Fn(&[GenerationProcessEvent]) -> bool,
) {
    let deadline = Instant::now() + timeout;
    while !predicate(&sink.events()) {
        assert!(
            Instant::now() < deadline,
            "{what} did not happen within {timeout:?}: {:?}",
            sink.events()
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn runtime_session_id(events: &[GenerationProcessEvent]) -> Option<String> {
    events.iter().find_map(|event| match event {
        GenerationProcessEvent::RuntimeSessionId(id) => Some(id.clone()),
        _ => None,
    })
}

/// Live tool round trips through the production adapter, in three phases, each a paid model
/// call: (1) a write tool call surfaces as an approval, is approved once, and the file lands;
/// (2) the same request under a deny policy runs no tool and leaves no file; (3) a long turn is
/// cancelled cooperatively, reported as `cancelled`, and the connection still serves the next
/// turn. Resuming across a host restart is covered separately: through
/// `session/load` and the agent still knows the earlier context (that last phase lives in
/// `live_resume_across_a_host_restart_through_the_acp_adapter`). Opt-in exactly like
/// `live_prompt_turn_completes_through_the_acp_adapter`.
#[test]
fn live_tool_approval_and_cancel_through_the_acp_adapter() {
    let Some(target) = live_prompt_target() else {
        return;
    };
    let agent_id = target.0.clone();

    // (1) Approval: the agent asks, the host answers once, the write happens.
    let workspace = TempDirectory::new("acp-live-approve");
    let (adapter, policy) = live_adapter(Effect::Ask, None);
    let started = Instant::now();
    let turn = adapter
        .start_generation(live_request(
            "session-live-approve",
            &target,
            workspace.path(),
            "Use your file write tool to create a file named approved.txt in the current \
             working directory whose entire content is the single word hello. Then reply DONE.",
            None,
        ))
        .expect("approval turn");
    let sink = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&turn.process_id, sink.clone())
        .expect("monitor");
    wait_until(
        &sink,
        Duration::from_secs(180),
        "an approval request",
        |events| {
            events.iter().any(|event| {
            matches!(event, GenerationProcessEvent::ToolLifecycle(tool) if tool.phase == ToolLifecyclePhase::AwaitingApproval)
        })
        },
    );
    let call_id = sink
        .events()
        .iter()
        .find_map(|event| match event {
            GenerationProcessEvent::ToolLifecycle(tool)
                if tool.phase == ToolLifecyclePhase::AwaitingApproval =>
            {
                Some(tool.call_id.clone())
            }
            _ => None,
        })
        .expect("call id");
    assert_eq!(
        policy.registered.lock().expect("lock").as_slice(),
        [call_id.as_str()],
        "exactly one pending approval was registered"
    );
    assert!(adapter
        .resolve(&turn.process_id, &call_id, ToolApprovalDecision::Approved)
        .expect("resolve"));
    assert!(
        !adapter
            .resolve(&turn.process_id, &call_id, ToolApprovalDecision::Approved)
            .expect("second resolve"),
        "an approval is consumed once"
    );
    let events = sink.wait_for_terminal(Duration::from_secs(180));
    let written = std::fs::read_to_string(workspace.path().join("approved.txt")).ok();
    eprintln!(
        "LIVE-APPROVE {agent_id}: {} events in {:?}; call={call_id}; file={written:?}; last={:?}",
        events.len(),
        started.elapsed(),
        events.last()
    );
    assert!(matches!(
        events.last(),
        Some(GenerationProcessEvent::Completed(_))
    ));
    assert!(
        written.as_deref().map(str::trim) == Some("hello"),
        "approved write did not land: {written:?}"
    );
    assert_eq!(adapter.release_session("session-live-approve"), 1);

    // (2) Deny: no approval is asked, the tool does not run, no file appears.
    let workspace = TempDirectory::new("acp-live-deny");
    let (adapter, policy) = live_adapter(Effect::Deny, None);
    let started = Instant::now();
    let turn = adapter
        .start_generation(live_request(
            "session-live-deny",
            &target,
            workspace.path(),
            "Use your file write tool to create a file named denied.txt in the current working \
             directory whose entire content is the single word hello. If the tool is refused, \
             do not try any other way; just reply REFUSED.",
            None,
        ))
        .expect("deny turn");
    let sink = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&turn.process_id, sink.clone())
        .expect("monitor");
    let events = sink.wait_for_terminal(Duration::from_secs(180));
    let denied_exists = workspace.path().join("denied.txt").exists();
    eprintln!(
        "LIVE-DENY {agent_id}: {} events in {:?}; asked={:?}; file_exists={denied_exists}; tokens={:?}; last={:?}",
        events.len(),
        started.elapsed(),
        policy.registered.lock().expect("lock"),
        tokens(&events),
        events.last()
    );
    assert!(
        policy.registered.lock().expect("lock").is_empty(),
        "a deny policy never asks"
    );
    assert!(!denied_exists, "the denied write must not land");
    assert!(matches!(
        events.last(),
        Some(GenerationProcessEvent::Completed(_))
    ));
    assert_eq!(adapter.release_session("session-live-deny"), 1);

    // (3) Cancel mid-stream, then reuse the same connection.
    let workspace = TempDirectory::new("acp-live-cancel");
    let (adapter, _) = live_adapter(Effect::Deny, None);
    let started = Instant::now();
    let turn = adapter
        .start_generation(live_request(
            "session-live-cancel",
            &target,
            workspace.path(),
            "Count from 1 to 400, one number per line, with no other text.",
            None,
        ))
        .expect("cancel turn");
    let sink = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&turn.process_id, sink.clone())
        .expect("monitor");
    wait_until(
        &sink,
        Duration::from_secs(120),
        "the first streamed token",
        |events| {
            events
                .iter()
                .any(|event| matches!(event, GenerationProcessEvent::Token(_)))
        },
    );
    assert!(adapter
        .stop_generation(&turn.process_id, ProcessStopInitiator::User)
        .expect("stop"));
    let events = sink.wait_for_terminal(Duration::from_secs(60));
    let external = runtime_session_id(&events).expect("runtime session id");
    eprintln!(
        "LIVE-CANCEL {agent_id}: {} events in {:?}; streamed {} chars before cancel; last={:?}",
        events.len(),
        started.elapsed(),
        tokens(&events).len(),
        events.last()
    );
    assert!(
        matches!(events.last(), Some(GenerationProcessEvent::Failed(failure)) if failure.safe_error.as_deref() == Some("cancelled")),
        "cancel must be reported as cancelled: {:?}",
        events.last()
    );
    let next = adapter
        .start_generation(live_request(
            "session-live-cancel",
            &target,
            workspace.path(),
            "Reply with exactly the word OK and nothing else.",
            Some(&external),
        ))
        .expect("next turn on the same connection");
    let sink = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&next.process_id, sink.clone())
        .expect("monitor");
    let events = sink.wait_for_terminal(Duration::from_secs(120));
    eprintln!(
        "LIVE-CANCEL-NEXT {agent_id}: tokens={:?}; last={:?}",
        tokens(&events),
        events.last()
    );
    assert!(matches!(
        events.last(),
        Some(GenerationProcessEvent::Completed(_))
    ));
    assert_eq!(
        adapter.release_session("session-live-cancel"),
        1,
        "one connection served both turns"
    );
}

/// Live resume across a host restart: the first adapter is shut down (connections terminated,
/// persisted binding kept), and a fresh adapter over the same store resumes the recorded
/// external session through `session/load` on a new process; the agent still holds the earlier
/// context. Opt-in exactly like the other live prompt tests; two paid calls.
#[test]
fn live_resume_across_a_host_restart_through_the_acp_adapter() {
    let Some(target) = live_prompt_target() else {
        return;
    };
    let agent_id = target.0.clone();
    let workspace = TempDirectory::new("acp-live-resume");
    let database = TempDirectory::new("acp-live-resume-db");
    let (adapter, _) = live_adapter(Effect::Deny, Some(database.path()));
    let started = Instant::now();
    let turn = adapter
        .start_generation(live_request(
            "session-live-resume",
            &target,
            workspace.path(),
            "The code word for this conversation is PINEAPPLE. Remember it and reply OK.",
            None,
        ))
        .expect("first resume turn");
    let sink = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&turn.process_id, sink.clone())
        .expect("monitor");
    let events = sink.wait_for_terminal(Duration::from_secs(120));
    let external = runtime_session_id(&events).expect("runtime session id");
    assert!(matches!(
        events.last(),
        Some(GenerationProcessEvent::Completed(_))
    ));
    // The host "restarts": every connection is terminated, the persisted binding survives
    // (`release_session` would delete it, because that means the session itself is gone), and a
    // fresh adapter over the same store is what the next turn goes through.
    assert_eq!(adapter.shutdown_all().len(), 1);
    drop(adapter);
    let (adapter, _) = live_adapter(Effect::Deny, Some(database.path()));
    let resumed = adapter
        .start_generation(live_request(
            "session-live-resume",
            &target,
            workspace.path(),
            "What is the code word for this conversation? Reply with just that word.",
            Some(&external),
        ))
        .expect("resumed turn");
    let sink = Arc::new(CapturingSink::default());
    adapter
        .monitor_generation(&resumed.process_id, sink.clone())
        .expect("monitor");
    let events = sink.wait_for_terminal(Duration::from_secs(120));
    let text = tokens(&events);
    eprintln!(
        "LIVE-RESUME {agent_id}: external session len {}; {} events in {:?}; tokens={text:?}; last={:?}",
        external.len(),
        events.len(),
        started.elapsed(),
        events.last()
    );
    assert!(matches!(
        events.last(),
        Some(GenerationProcessEvent::Completed(_))
    ));
    assert!(
        text.to_uppercase().contains("PINEAPPLE"),
        "the resumed session lost its context: {text:?}"
    );
    assert_eq!(adapter.release_session("session-live-resume"), 1);
}
