//! One OnePiece generation over the real memory stack.
//!
//! Real v2 files, the SQLite projection and FTS index, the governed bridge, the Context Engine's
//! memory source, the injected index page, the body selector, and `recall` -- every surface
//! deciding eligibility from the one read context frozen for the generation. The model is a
//! scripted local HTTP endpoint; the only other stand-ins are the embedder (fixed vectors, so it
//! can be counted) and the desktop-shell ports the settings service needs.
//!
//! The corpus is built so that every excluded record matches the query at least as well as the
//! admitted one: an exclusion that leaked through any surface would show up as text in a request
//! body, which is what the assertions read.

use super::personalization_bridge::GovernedPersonalizationAdapter;
use super::retrieval::{
    DeferredAgentRetrieval, GovernedEmbeddingEgressGuard, GovernedMemoryIndexSource,
};
use crate::contexts::agent_runtime::application::{
    AgentChatConfiguration, AgentClockPort, AgentLaunchView, AgentLog, AgentLoggingPort,
    AgentMcpToolPort, AgentMessage, AgentPermissionPort, AgentProcessEventSink,
    AgentProcessGateway, AgentRetrievalPort, AgentRuntimeApplicationError, AgentSession,
    AgentSkillPort, AgentToolCallOutcome, AgentView, AgentWorkspaceMutation,
    AgentWorkspaceMutationPort, ApiAgentGateway, ApiCredentialPort, ApiProviderConfig,
    BoundSkillPrompt, CliProfileSnapshot, ContextEngineService, ContextManifestRepository,
    ConversationHistoryPort, GenerationProcessEvent, GenerationProcessRequest,
    RegisterApiAgentInput, RunnerSelection, ToolDefinition, UpdateApiAgentInput,
    INTERFACE_FORMAT_OPENAI_COMPATIBLE,
};
use crate::contexts::agent_runtime::domain::{
    AgentAvailability, AgentDefinition, AgentLifecycle, AgentOrigin, AutomaticCompactionMode,
    ContextSourceKind, ContextSourceOutcome, InteractionMode,
};
use crate::contexts::agent_runtime::infrastructure::{
    ExplicitReferenceContextSource, MonotonicContextEngineClock,
    NativeAgentCoreInstructionsAdapter, RetrievalContextSource, RuntimeAgentApiAdapter,
    SqliteContextManifestRepository, UnifiedContextEngineDiagnostics,
};
use crate::contexts::desktop::api::DesktopSettingsApi;
use crate::contexts::desktop::application::{
    DesktopClockPort, DesktopEnvironmentApplicationService, DesktopLocalePort,
    DesktopLogDirectoryPort, DesktopNetworkProxyPort, DesktopSettingsApplicationError,
    DesktopSettingsApplicationService, DesktopStartupPort,
};
use crate::contexts::desktop::domain::{
    ApplicationLanguage, NetworkProxyPreferences, StartupPreference,
};
use crate::contexts::desktop::infrastructure::{
    DesktopDirectoryAdapter, FolderOpenerService, PlatformNodeInfoAdapter,
    RuntimeNetworkProxyActionsAdapter, SqliteDesktopSettingsRepository,
    UnifiedClientLoggingAdapter,
};
use crate::contexts::execution_observability::api::CapturePolicy;
use crate::contexts::execution_observability::application::ExecutionIdentityPort;
use crate::contexts::execution_observability::infrastructure::RandomExecutionIdentity;
use crate::contexts::permissions::api::{Action, Effect, Resource};
use crate::contexts::personalization::api::{build_for_tests, PersonalizationApi};
use crate::contexts::personalization::application::{
    legacy_workspace_request, ClockPort, CreateMemoryInput, MemoryApplicationService,
    MigrationStatePort, PersonalizationApplicationError, PolicyRepository, RetrievalIndexPort,
    WorkspaceIdentityPort, WorkspaceIdentityResolver,
};
use crate::contexts::personalization::domain::{
    AgentId, MemoryAudience, MemoryId, MemoryProvenance, MemoryRecord, MemoryScope,
    MemorySensitivity, MemorySource as GovernedSource, MemoryStatus, MemoryType as GovernedType,
    MigrationPhase, MigrationState, WorkspaceKey,
};
use crate::contexts::personalization::infrastructure::{
    SqliteMigrationState, SqlitePolicyRepository,
};
use crate::contexts::retrieval::api::{RetrievalApi, RetrievalWorkerSignal};
use crate::contexts::retrieval::application::{
    EmbeddingFailure, EmbeddingPort, IndexingService, RetrievalConfigurationRepository,
    RetrievalDocumentRepository, SearchService,
};
use crate::contexts::retrieval::infrastructure::{
    SqliteRetrievalConfigurationRepository, SqliteRetrievalDocumentRepository,
};
use crate::platform::database::NativeDatabase;
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const MODEL: &str = "fixture-embedding-model";
const QUERY: &str = "package manager";
const TASK: &str = "Which package manager does this project use?";

// ---------------------------------------------------------------------------------------------
// Stand-ins. Each is the smallest thing that satisfies a port the generation needs; none of them
// decides anything about memory.
// ---------------------------------------------------------------------------------------------

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 9, 9, 0, 0).unwrap()
}

struct GovernedClock;
impl ClockPort for GovernedClock {
    fn now(&self) -> DateTime<Utc> {
        now()
    }
}

/// The personalization side's own retrieval publication is not under test here: the index is
/// built the way the worker builds it, through `IndexingService::reconcile` over the governed
/// maintenance source.
#[derive(Default)]
struct UnusedIndex;
impl RetrievalIndexPort for UnusedIndex {
    fn upsert(&self, _record: &MemoryRecord) -> Result<(), PersonalizationApplicationError> {
        Ok(())
    }
    fn revoke(&self, _id: &MemoryId) -> Result<(), PersonalizationApplicationError> {
        Ok(())
    }
    fn revoke_all(&self, _ids: &[MemoryId]) -> Result<usize, PersonalizationApplicationError> {
        Ok(0)
    }
    fn indexed_ids(&self) -> Result<Vec<MemoryId>, PersonalizationApplicationError> {
        Ok(Vec::new())
    }
}

/// Every text handed to the embedder, so the test can prove which bodies left for it.
#[derive(Default)]
struct CountingEmbedder {
    inputs: Mutex<Vec<String>>,
}
impl EmbeddingPort for CountingEmbedder {
    fn embed(&self, _model: &str, inputs: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingFailure> {
        self.inputs
            .lock()
            .expect("inputs")
            .extend(inputs.iter().cloned());
        Ok(inputs.iter().map(|_| vec![1.0, 0.0, 0.0]).collect())
    }
}

struct DesktopNoop;
impl DesktopClockPort for DesktopNoop {
    fn now(&self) -> String {
        "2026-09-09T09:00:00.000Z".to_string()
    }
}
impl DesktopNetworkProxyPort for DesktopNoop {
    fn apply(
        &self,
        _preferences: &NetworkProxyPreferences,
    ) -> Result<(), DesktopSettingsApplicationError> {
        Ok(())
    }
}
impl DesktopLogDirectoryPort for DesktopNoop {
    fn validate(&self, _path: &str) -> Result<(), DesktopSettingsApplicationError> {
        Ok(())
    }
    fn activate(&self, _path: &str) -> Result<(), DesktopSettingsApplicationError> {
        Ok(())
    }
}
impl DesktopStartupPort for DesktopNoop {
    fn apply(&self, _preference: StartupPreference) -> Result<(), DesktopSettingsApplicationError> {
        Ok(())
    }
}
impl DesktopLocalePort for DesktopNoop {
    fn apply(&self, _language: ApplicationLanguage) -> Result<(), DesktopSettingsApplicationError> {
        Ok(())
    }
}

struct FixtureCredentials;
impl ApiCredentialPort for FixtureCredentials {
    fn store(&self, _agent_id: &str, _api_key: &str) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
    fn fetch(&self, _agent_id: &str) -> Result<Option<String>, AgentRuntimeApplicationError> {
        Ok(Some("sk-fixture".to_string()))
    }
    fn remove(&self, _agent_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
}

struct FixtureEndpoint(String);
impl ApiAgentGateway for FixtureEndpoint {
    fn register(
        &self,
        _agent_id: &str,
        _input: &RegisterApiAgentInput,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        unimplemented!("not exercised")
    }
    fn provider_config(
        &self,
        _agent_id: &str,
    ) -> Result<Option<ApiProviderConfig>, AgentRuntimeApplicationError> {
        Ok(Some(ApiProviderConfig {
            source_provider_id: None,
            model_id: "fixture-model".to_string(),
            interface_format: INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string(),
            base_url: Some(self.0.clone()),
            auto_approve_tools: true,
        }))
    }
    fn update(
        &self,
        _agent_id: &str,
        _input: &UpdateApiAgentInput,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        unimplemented!("not exercised")
    }
    fn delete(&self, _agent_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        unimplemented!("not exercised")
    }
}

/// The API runtime reads the conversation from history, so the task is the one user turn.
struct TaskHistory;
impl ConversationHistoryPort for TaskHistory {
    fn recent_messages(
        &self,
        session_id: &str,
        _limit: i64,
    ) -> Result<Vec<AgentMessage>, AgentRuntimeApplicationError> {
        Ok(vec![AgentMessage {
            id: format!("{session_id}-message"),
            session_id: session_id.to_string(),
            speaker_seat_id: None,
            seat_index: None,
            role: "user".to_string(),
            content: TASK.to_string(),
            status: "completed".to_string(),
            tool_use: Vec::new(),
            thinking_content: None,
            rich_blocks: Vec::new(),
            token_usage: None,
            file_references: Vec::new(),
            error: None,
            created_at: "2026-09-09T09:00:00Z".to_string(),
            updated_at: "2026-09-09T09:00:00Z".to_string(),
            session_sequence: 1,
            execution_run_id: None,
        }])
    }
}

struct QuietLogging;
impl AgentLoggingPort for QuietLogging {
    fn record(&self, _log: AgentLog) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
}

struct RuntimeClock;
impl AgentClockPort for RuntimeClock {
    fn now(&self) -> String {
        "2026-09-09T09:00:00Z".to_string()
    }
}

struct NoSkills;
impl AgentSkillPort for NoSkills {
    fn bound_skill_prompts(
        &self,
        _agent_id: &str,
        _workspace_path: Option<&str>,
    ) -> Result<Vec<BoundSkillPrompt>, AgentRuntimeApplicationError> {
        Ok(Vec::new())
    }
}

struct NoMcp;
impl AgentMcpToolPort for NoMcp {
    fn catalog_entries(
        &self,
        _project_path: &str,
    ) -> Result<Vec<ToolDefinition>, AgentRuntimeApplicationError> {
        Ok(Vec::new())
    }
    fn call_tool(
        &self,
        _project_path: &str,
        name: &str,
        _arguments: &Value,
        _cancellation: Arc<AtomicBool>,
    ) -> AgentToolCallOutcome {
        AgentToolCallOutcome {
            output: format!("no MCP tool named {name}"),
            is_error: true,
        }
    }
}

/// Everything allowed. Permissions are not what decides memory eligibility, and an `Ask` would
/// only stall the scripted turn on an approval nobody is there to give.
struct AllowAll;
impl AgentPermissionPort for AllowAll {
    fn evaluate(
        &self,
        _agent_id: &str,
        _action: Action,
        _resource: Resource,
        _session_id: &str,
        _generation_id: &str,
        _project_key: &str,
    ) -> Effect {
        Effect::Allow
    }
    fn create_pending_approval(
        &self,
        _agent_id: &str,
        _action: Action,
        _resource: Resource,
        _session_id: &str,
        _generation_id: &str,
        _call_id: &str,
        _project_key: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
}

struct DropMutations;
impl AgentWorkspaceMutationPort for DropMutations {
    fn publish(&self, _mutation: AgentWorkspaceMutation) {}
}

#[derive(Default)]
struct EventLog(Mutex<Vec<GenerationProcessEvent>>);
impl AgentProcessEventSink for EventLog {
    fn handle(&self, event: GenerationProcessEvent) -> Result<(), AgentRuntimeApplicationError> {
        self.0.lock().expect("events").push(event);
        Ok(())
    }
}

impl EventLog {
    fn wait_until_terminal(&self) -> GenerationProcessEvent {
        let started = Instant::now();
        loop {
            {
                let events = self.0.lock().expect("events");
                if let Some(terminal) = events.iter().find(|event| {
                    matches!(
                        event,
                        GenerationProcessEvent::Completed(_) | GenerationProcessEvent::Failed(_)
                    )
                }) {
                    return terminal.clone();
                }
            }
            assert!(
                started.elapsed() < Duration::from_secs(20),
                "the generation never reached a terminal event"
            );
            thread::sleep(Duration::from_millis(25));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The scripted model.
// ---------------------------------------------------------------------------------------------

fn sse(events: &[String]) -> String {
    events
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect()
}

fn text_turn(text: &str) -> String {
    sse(&[
        format!(
            r#"{{"choices":[{{"index":0,"delta":{{"content":{}}},"finish_reason":null}}]}}"#,
            Value::String(text.to_string())
        ),
        "[DONE]".to_string(),
    ])
}

fn recall_turn(query: &str) -> String {
    let arguments = Value::String(serde_json::json!({ "query": query }).to_string());
    sse(&[
        format!(
            r#"{{"choices":[{{"index":0,"delta":{{"tool_calls":[{{"index":0,"id":"call_recall","type":"function","function":{{"name":"recall","arguments":{arguments}}}}}]}},"finish_reason":null}}]}}"#
        ),
        "[DONE]".to_string(),
    ])
}

/// Answers `bodies.len()` requests in order on one local address and hands back what each
/// request carried, so the assertions read what the model was actually shown.
fn scripted_model(bodies: Vec<String>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture");
    let address = listener.local_addr().expect("fixture address");
    let handle = thread::spawn(move || {
        bodies
            .into_iter()
            .map(|body| {
                let (mut stream, _) = listener.accept().expect("accept fixture request");
                let request = read_request(&mut stream);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("write fixture response");
                String::from_utf8_lossy(&request).into_owned()
            })
            .collect()
    });
    (format!("http://{address}"), handle)
}

fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let count = stream.read(&mut buffer).expect("read fixture request");
        if count == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..count]);
        let Some(header_end) = request.windows(4).position(|value| value == b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&request[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        if request.len() >= header_end + 4 + content_length {
            break;
        }
    }
    request
}

// ---------------------------------------------------------------------------------------------
// The real stack.
// ---------------------------------------------------------------------------------------------

struct Corpus {
    global_all: MemoryRecord,
    workspace_all: MemoryRecord,
    other_workspace_all: MemoryRecord,
    global_agent_b: MemoryRecord,
}

struct Stack {
    _root: TempDir,
    folder: String,
    personalization: PersonalizationApi,
    memories: Arc<MemoryApplicationService>,
    settings: DesktopSettingsApi,
    database: NativeDatabase,
    retrieval_api: RetrievalApi,
    documents: Arc<dyn RetrievalDocumentRepository>,
    retrieval: Arc<DeferredAgentRetrieval>,
    embedder: Arc<CountingEmbedder>,
    _wakeups: std::sync::mpsc::Receiver<()>,
}

fn workspace_key_of(folder: &str) -> WorkspaceKey {
    let request = legacy_workspace_request(folder).expect("workspace request");
    WorkspaceIdentityResolver::for_this_platform()
        .resolve(&request)
        .expect("resolve workspace")
        .expect("a workspace identity")
        .key()
        .clone()
}

fn stack(label: &str) -> Stack {
    let root = TempDir::with_prefix(format!("memory-read-generation-{label}-")).expect("root");
    let database = NativeDatabase::new(root.path().to_path_buf()).expect("database");
    let folder = root.path().join("w1");
    std::fs::create_dir_all(&folder).expect("workspace folder");

    let (personalization, memories) = build_for_tests(
        root.path().join("memory"),
        database.clone(),
        Arc::new(UnusedIndex),
        Arc::new(GovernedClock),
    );
    SqliteMigrationState::new(database.clone())
        .save(&MigrationState {
            generation: 1,
            phase: MigrationPhase::Ready,
            started_at: Some(now()),
            completed_at: Some(now()),
            legacy_rows_migrated_at: Some(now()),
            last_error_code: None,
            repair_required: false,
            last_reconciled_at: None,
        })
        .expect("mark memory ready");
    SqlitePolicyRepository::new(database.clone())
        .seed_default_global(now())
        .expect("seed global policy");

    let settings_repository = SqliteDesktopSettingsRepository::new(database.clone());
    let settings = DesktopSettingsApi::new(
        DesktopSettingsApplicationService::new(
            Arc::new(settings_repository.clone()),
            Arc::new(DesktopNoop),
            Arc::new(DesktopNoop),
            Arc::new(DesktopNoop),
            Arc::new(DesktopNoop),
            Arc::new(DesktopNoop),
            root.path().to_string_lossy().to_string(),
        ),
        DesktopEnvironmentApplicationService::new(
            Arc::new(DesktopDirectoryAdapter::new(database.clone())),
            Arc::new(PlatformNodeInfoAdapter),
            Arc::new(RuntimeNetworkProxyActionsAdapter),
            Arc::new(UnifiedClientLoggingAdapter),
        ),
        FolderOpenerService::new(settings_repository),
    );

    let documents: Arc<dyn RetrievalDocumentRepository> =
        Arc::new(SqliteRetrievalDocumentRepository::new(database.clone()));
    let configuration: Arc<dyn RetrievalConfigurationRepository> = Arc::new(
        SqliteRetrievalConfigurationRepository::new(database.clone()),
    );
    configuration
        .save("fixture-profile", MODEL)
        .expect("configure embeddings");
    let embedder = Arc::new(CountingEmbedder::default());
    let (signal, wakeups) = RetrievalWorkerSignal::channel();
    let retrieval_api = RetrievalApi::new(
        Arc::new(SearchService::new(
            configuration.clone(),
            documents.clone(),
            embedder.clone(),
        )),
        documents.clone(),
        configuration,
        signal,
    );
    let retrieval = Arc::new(DeferredAgentRetrieval::default());
    retrieval.bind(retrieval_api.clone());
    retrieval.bind_personalization(personalization.clone());

    Stack {
        folder: folder.to_string_lossy().to_string(),
        _root: root,
        personalization,
        memories,
        settings,
        database,
        retrieval_api,
        documents,
        retrieval,
        embedder,
        _wakeups: wakeups,
    }
}

impl Stack {
    fn seed(
        &self,
        name: &str,
        content: &str,
        scope: MemoryScope,
        audience: MemoryAudience,
    ) -> MemoryRecord {
        self.memories
            .create(CreateMemoryInput {
                name: name.to_string(),
                description: format!("what {name} records"),
                memory_type: GovernedType::Project,
                content: content.to_string(),
                scope,
                audience,
                status: MemoryStatus::Active,
                source: GovernedSource::ExplicitUser,
                provenance: MemoryProvenance::default(),
                sensitivity: MemorySensitivity::Normal,
            })
            .expect("seed memory")
            .record
    }

    /// Four records that all answer "package manager"; only two are readable by Agent
    /// `onepiece` in workspace W1, and only one of those two carries the word the model needs.
    fn seed_corpus(&self) -> Corpus {
        let other_folder = self._root.path().join("w2");
        std::fs::create_dir_all(&other_folder).expect("other workspace folder");
        let w1 = workspace_key_of(&self.folder);
        let w2 = workspace_key_of(&other_folder.to_string_lossy());
        assert_ne!(w1, w2);
        Corpus {
            global_all: self.seed(
                "release-train",
                "The release train ships every Tuesday.",
                MemoryScope::Global,
                MemoryAudience::AllAgents,
            ),
            workspace_all: self.seed(
                "package-manager",
                "This project uses pnpm as its package manager.",
                MemoryScope::Workspace { workspace_key: w1 },
                MemoryAudience::AllAgents,
            ),
            other_workspace_all: self.seed(
                "other-package-manager",
                "That other project uses yarn as its package manager.",
                MemoryScope::Workspace { workspace_key: w2 },
                MemoryAudience::AllAgents,
            ),
            global_agent_b: self.seed(
                "cargo-preference",
                "Agent B prefers cargo as its package manager.",
                MemoryScope::Global,
                MemoryAudience::SelectedAgents {
                    agent_ids: vec![AgentId::parse("agent-b").expect("agent id")],
                },
            ),
        }
    }

    /// The worker's own path: reconcile the governed source into the index, then dispatch the
    /// embedding batch through the egress guard.
    fn index(&self) {
        let indexing = IndexingService::new(
            self.documents.clone(),
            Arc::new(GovernedMemoryIndexSource {
                personalization: self.personalization.clone(),
            }),
            self.embedder.clone(),
        )
        .with_egress_guard(Arc::new(GovernedEmbeddingEgressGuard {
            personalization: self.personalization.clone(),
        }));
        indexing.reconcile().expect("reconcile");
        indexing.process_pending_batch(MODEL).expect("embed batch");
    }

    fn runtime(&self, model_address: &str) -> RuntimeAgentApiAdapter {
        let bridge = GovernedPersonalizationAdapter::without_sessions(
            self.personalization.clone(),
            self.settings.clone(),
        );
        let retrieval: Arc<dyn AgentRetrievalPort> = self.retrieval.clone();
        let logging: Arc<dyn AgentLoggingPort> = Arc::new(QuietLogging);
        let clock: Arc<dyn AgentClockPort> = Arc::new(RuntimeClock);
        let engine = ContextEngineService::new(
            vec![
                Arc::new(ExplicitReferenceContextSource),
                Arc::new(RetrievalContextSource::memory(retrieval.clone())),
            ],
            Arc::new(SqliteContextManifestRepository::new(self.database.clone())),
            Arc::new(UnifiedContextEngineDiagnostics::new(
                logging.clone(),
                clock.clone(),
            )),
            Arc::new(MonotonicContextEngineClock::default()),
        );
        RuntimeAgentApiAdapter::new_without_code_intelligence(
            Arc::new(FixtureCredentials),
            Arc::new(FixtureEndpoint(model_address.to_string())),
            Arc::new(TaskHistory),
            logging,
            clock,
            Arc::new(NoSkills),
            Arc::new(NativeAgentCoreInstructionsAdapter),
            Arc::new(NoMcp),
            Arc::new(AllowAll),
            retrieval,
            Arc::new(DropMutations),
            Arc::new(bridge),
        )
        .with_context_engine(Arc::new(engine))
    }

    fn other_folder(&self) -> String {
        self._root.path().join("w2").to_string_lossy().to_string()
    }

    fn request(&self, session_id: &str, mode: &str) -> GenerationProcessRequest {
        self.request_in(session_id, mode, &self.folder)
    }

    fn request_in(&self, session_id: &str, mode: &str, folder: &str) -> GenerationProcessRequest {
        GenerationProcessRequest {
            execution_context: RandomExecutionIdentity.next_context(
                CapturePolicy::MetadataOnly,
                0.0,
                false,
            ),
            session: AgentSession {
                id: session_id.to_string(),
                agent_id: "onepiece".to_string(),
                seats: Vec::new(),
                interaction_mode: InteractionMode::Api,
                personalization_mode: mode.to_string(),
                lifecycle: AgentLifecycle::Running,
                folder: Some(folder.to_string()),
                runtime_session_id: None,
                archived: false,
                read_only: false,
                loop_ownership: None,
            },
            agent: AgentView {
                id: "onepiece".to_string(),
                display_name: "OnePiece".to_string(),
                provider: "OpenAI-compatible".to_string(),
                managed_sdk_dependency_id: None,
                launch: AgentLaunchView {
                    kind: "api".to_string(),
                    command: None,
                    url: None,
                    executable_name: None,
                },
                supported_interaction_modes: vec![InteractionMode::Api],
                availability: AgentAvailability::Available,
                unavailable_reason: None,
                capability_tags: vec!["api".to_string()],
                origin: AgentOrigin::User,
            },
            message_id: format!("{session_id}-message"),
            operation_id: format!("{session_id}-generation"),
            configuration: AgentChatConfiguration {
                agent_id: "onepiece".to_string(),
                interaction_mode: InteractionMode::Api,
                execution_mode: "inherit".to_string(),
                provider_id: None,
                model_id: None,
                reasoning_depth: None,
                streaming: true,
                thinking: false,
                long_context: false,
            },
            effective_prompt: TASK.to_string(),
            file_references: Vec::new(),
            automatic_compaction: AutomaticCompactionMode::Automatic,
            role_briefing: None,
            cli_profile: CliProfileSnapshot {
                executable: String::new(),
                global_args: Vec::new(),
                invocation_args: Vec::new(),
                env: Default::default(),
            },
            interactive: false,
            runner: RunnerSelection::local(),
            endpoint_profile: None,
            resume_thread_id: None,
            seat_id: None,
        }
    }

    fn manifest(
        &self,
        generation_id: &str,
    ) -> Option<crate::contexts::agent_runtime::domain::ContextEvidenceManifest> {
        SqliteContextManifestRepository::new(self.database.clone())
            .get(generation_id)
            .expect("read manifest")
    }

    fn run(
        &self,
        request: GenerationProcessRequest,
        script: Vec<String>,
    ) -> (Vec<String>, GenerationProcessEvent) {
        let (address, model) = scripted_model(script);
        let runtime = self.runtime(&address);
        let started = runtime.start_generation(request).expect("start generation");
        let events = Arc::new(EventLog::default());
        runtime
            .monitor_generation(&started.process_id, events.clone())
            .expect("monitor generation");
        let requests = model.join().expect("scripted model");
        let terminal = events.wait_until_terminal();
        (requests, terminal)
    }
}

fn assert_never_shown(requests: &[String], corpus: &Corpus) {
    for request in requests {
        assert!(
            !request.contains(&corpus.other_workspace_all.content),
            "another workspace's body reached the model: {request}"
        );
        assert!(
            !request.contains(&corpus.global_agent_b.content),
            "another Agent's audience-restricted body reached the model: {request}"
        );
        assert!(
            !request.contains(corpus.other_workspace_all.id.as_str()),
            "another workspace's id reached the model: {request}"
        );
        assert!(
            !request.contains(corpus.global_agent_b.id.as_str()),
            "another Agent's memory id reached the model: {request}"
        );
    }
}

/// MR-01 / MR-03 / MR-11 (the real path) with MR-22 / MR-23 (the index behind it): a standard
/// session in W1 sees the global and the W1 record on the index page, at the selector, in the
/// Context Engine's evidence and through `recall`; the W2 and Agent-B records match the query
/// just as well and reach none of those surfaces. No scoped body ever left for the embedder.
#[test]
fn a_standard_generation_reads_exactly_its_scope_on_every_surface_over_the_real_stack() {
    let stack = stack("standard");
    let corpus = stack.seed_corpus();
    stack.index();

    let status = stack.retrieval_api.index_status().expect("index status");
    assert_eq!(
        status.indexed, 1,
        "only the public global record is embedded"
    );
    assert_eq!(
        status.keyword_only, 3,
        "scoped and audience records stay keyword-only"
    );
    {
        let embedded = stack.embedder.inputs.lock().expect("inputs");
        assert_eq!(
            embedded.as_slice(),
            std::slice::from_ref(&corpus.global_all.content)
        );
    }

    let script = vec![
        // The body selector: asked with the index manifest, answers with the W1 id.
        text_turn(&format!(
            "[{}]",
            Value::String(corpus.workspace_all.file_name())
        )),
        // The turn: the model asks memory for the package manager ...
        recall_turn(QUERY),
        // ... and answers once it has the tool result.
        text_turn("This project uses pnpm."),
    ];
    let (requests, terminal) = stack.run(stack.request("session-standard", "standard"), script);

    assert!(
        matches!(terminal, GenerationProcessEvent::Completed(_)),
        "the generation did not complete"
    );
    assert_eq!(requests.len(), 3, "selector, tool turn, final turn");
    assert_never_shown(&requests, &corpus);

    // I -- the selector manifest and the injected index page name only eligible records.
    let selector = &requests[0];
    assert!(selector.contains(corpus.workspace_all.id.as_str()));
    assert!(selector.contains(corpus.global_all.id.as_str()));
    let turn = &requests[1];
    assert!(turn.contains("## Memory"), "the index page is injected");
    assert!(turn.contains(corpus.workspace_all.id.as_str()));
    assert!(turn.contains(corpus.global_all.id.as_str()));
    // B -- the selected W1 body is delivered, by id.
    assert!(
        turn.contains("pnpm"),
        "the selected workspace body is injected: {turn}"
    );
    // C -- the Context Engine's memory source ran under the same context: it recorded the
    // memory source as ready in this generation's manifest, and embedded the task to search.
    let manifest = stack
        .manifest("session-standard-generation")
        .expect("a manifest for the generation");
    assert_eq!(
        manifest.source_outcomes.get(&ContextSourceKind::Memory),
        Some(&ContextSourceOutcome::Ready)
    );
    // R -- `recall` is offered, and its result carries the W1 body only.
    assert!(
        turn.contains("\"recall\""),
        "recall is in the catalog: {turn}"
    );
    let after_recall = &requests[2];
    assert!(
        after_recall.contains("pnpm"),
        "the recall result reached the follow-up turn: {after_recall}"
    );

    // The query embedding went out (the global record is vector-searchable); no scoped body did.
    let embedded = stack.embedder.inputs.lock().expect("inputs");
    assert!(
        embedded.iter().any(|input| input == TASK),
        "the engine embedded the task"
    );
    assert!(
        embedded.iter().any(|input| input == QUERY),
        "recall embedded its query"
    );
    for input in embedded.iter() {
        assert!(!input.contains("pnpm") && !input.contains("yarn") && !input.contains("cargo"));
    }
}

/// MR-04 over the real stack: a temporary session keeps generating and touches no memory
/// surface at all -- no index page, no selector call, no `recall` in the catalog, no Context
/// Engine evidence, and not one query embedding.
#[test]
fn a_temporary_generation_completes_without_touching_any_memory_surface_over_the_real_stack() {
    let stack = stack("temporary");
    let corpus = stack.seed_corpus();
    stack.index();
    let embedded_before = stack.embedder.inputs.lock().expect("inputs").len();

    let (requests, terminal) = stack.run(
        stack.request("session-temporary", "temporary"),
        vec![text_turn("I have no memory of this project.")],
    );

    assert!(
        matches!(terminal, GenerationProcessEvent::Completed(_)),
        "the generation did not complete"
    );
    assert_eq!(requests.len(), 1, "no selector call, no tool turn");
    assert_never_shown(&requests, &corpus);
    let turn = &requests[0];
    assert!(!turn.contains("## Memory"));
    assert!(!turn.contains("context-evidence"));
    assert!(!turn.contains("\"recall\""));
    assert!(!turn.contains("pnpm"));
    assert!(!turn.contains(corpus.workspace_all.id.as_str()));
    assert!(!turn.contains(corpus.global_all.id.as_str()));
    assert_eq!(
        stack.embedder.inputs.lock().expect("inputs").len(),
        embedded_before,
        "no query was embedded for a session that may not read"
    );
    let manifest = stack
        .manifest("session-temporary-generation")
        .expect("a manifest for the generation");
    assert_eq!(
        manifest.source_outcomes.get(&ContextSourceKind::Memory),
        Some(&ContextSourceOutcome::Unavailable),
        "the memory source was refused rather than run"
    );
}

/// MR-20 and 5.6 over the real stack: after W1 has been read on this host, a session in W2 sees
/// W2's record on every surface and W1's on none. Nothing minted for the first session --
/// its context, its surfaced markers, its recall relation -- carries over to the second.
#[test]
fn a_session_in_another_workspace_never_reuses_the_previous_workspace_authorization() {
    let stack = stack("other-workspace");
    let corpus = stack.seed_corpus();
    stack.index();

    let (first, _) = stack.run(
        stack.request("session-w1", "standard"),
        vec![
            text_turn(&format!(
                "[{}]",
                Value::String(corpus.workspace_all.file_name())
            )),
            recall_turn(QUERY),
            text_turn("pnpm."),
        ],
    );
    assert!(first[2].contains("pnpm"), "W1 was read first: {}", first[2]);

    let (requests, terminal) = stack.run(
        stack.request_in("session-w2", "standard", &stack.other_folder()),
        vec![
            text_turn(&format!(
                "[{}]",
                Value::String(corpus.other_workspace_all.file_name())
            )),
            recall_turn(QUERY),
            text_turn("yarn."),
        ],
    );

    assert!(
        matches!(terminal, GenerationProcessEvent::Completed(_)),
        "the generation did not complete"
    );
    assert_eq!(requests.len(), 3);
    for request in &requests {
        assert!(
            !request.contains("pnpm"),
            "W1's body reached the W2 session: {request}"
        );
        assert!(!request.contains(corpus.workspace_all.id.as_str()));
        assert!(!request.contains(&corpus.global_agent_b.content));
        assert!(!request.contains(corpus.global_agent_b.id.as_str()));
    }
    assert!(requests[0].contains(corpus.other_workspace_all.id.as_str()));
    assert!(requests[1].contains(corpus.other_workspace_all.id.as_str()));
    assert!(requests[1].contains(corpus.global_all.id.as_str()));
    assert!(
        requests[1].contains("yarn"),
        "W2's selected body is injected"
    );
    assert!(
        requests[2].contains("yarn"),
        "W2's recall result reaches the follow-up turn"
    );
}
