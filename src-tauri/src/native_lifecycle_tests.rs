use crate::contexts::agent_runtime::application::{
    AgentCliProfileGateway, AgentClockPort, AgentEvent, AgentEventPort, AgentLog, AgentLoggingPort,
    AgentRegistryRepository, AgentRuntimeApplicationError, AgentSessionGateway,
    AgentTerminalApplicationPorts, AgentTerminalApplicationService, AgentTerminalEvent,
    AgentTerminalEventPort, AgentTerminalGateway, AgentTerminalInputRequest,
    AgentTerminalProcessRequest, AgentTerminalSession, AgentTerminalSize, AgentTerminalState,
    CliProfileSnapshot, OpenAgentTerminalRequest, ResizeAgentTerminalRequest,
    StopAgentTerminalRequest,
};
use crate::contexts::agent_runtime::domain::{
    AgentAvailability, AgentDefinition, AgentDefinitionInput, AgentLifecycle,
    AvailabilityAssessment, InteractionMode, LaunchMetadata,
};
use crate::contexts::agent_runtime::infrastructure::SessionsAgentRuntimeAdapter;
use crate::contexts::operations::api::OperationsApi;
use crate::contexts::operations::application::{
    ApplicationError as OperationsApplicationError, OperationClock, OperationIdGenerator,
    OperationRepository, OperationService,
};
use crate::contexts::operations::domain::{OperationStatus, OperationTask};
use crate::contexts::sessions::api::{NewSessionRequest, NewSessionWorkspace, SessionsApi};
use crate::contexts::sessions::application::{
    ChatConfigurationValues, CreatedSessionWorktree, NewRemoteWorkspace, SessionApplicationLog,
    SessionApplicationPorts, SessionChatProfilePort, SessionClockPort, SessionCreationContextPort,
    SessionFileContentPort, SessionIdentityPort, SessionLoggingPort, SessionProject, SessionRecord,
    SessionRemoteWorkspace, SessionRuntimePort, SessionsApplicationError,
    SessionsApplicationService, UsageStatisticsRange,
};
use crate::contexts::sessions::domain::{SessionActivation, SessionOwner};
use crate::contexts::sessions::infrastructure::{
    SessionOperationAdapter, SqliteSessionsRepository,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

struct LifecycleHarness {
    _directory: TempDirectory,
    database: NativeDatabase,
    operations: OperationsApi,
    sessions: SessionsApi,
    terminals: AgentTerminalApplicationService,
    doubles: Arc<LifecycleDoubles>,
}

impl LifecycleHarness {
    fn new() -> Self {
        let directory = TempDirectory::new("native-agent-lifecycle");
        let database =
            NativeDatabase::new(directory.path().to_path_buf()).expect("temporary database");
        let repository = Arc::new(SqliteSessionsRepository::new(database.clone()));
        let doubles = Arc::new(LifecycleDoubles::default());
        let operations = OperationsApi::new(OperationService::new(
            Arc::new(LifecycleOperationRepository::default()),
            Arc::new(LifecycleOperationClock),
            Arc::new(LifecycleOperationIds),
        ));
        let service = SessionsApplicationService::new(SessionApplicationPorts {
            sessions: repository.clone(),
            messages: repository.clone(),
            categories: repository.clone(),
            configurations: repository.clone(),
            usage: repository.clone(),
            transactions: repository,
            clock: doubles.clone(),
            identities: doubles.clone(),
            files: doubles.clone(),
            operations: Arc::new(SessionOperationAdapter::new(operations.clone())),
            logging: doubles.clone(),
            chat_profiles: doubles.clone(),
            creation: doubles.clone(),
            runtime: doubles.clone(),
        });
        let sessions = SessionsApi::new(service);
        let session_gateway: Arc<dyn AgentSessionGateway> =
            Arc::new(SessionsAgentRuntimeAdapter::new(sessions.clone()));
        doubles.bind_agent_sessions(session_gateway.clone());
        let terminals = AgentTerminalApplicationService::new(AgentTerminalApplicationPorts {
            registry: doubles.clone(),
            sessions: session_gateway,
            cli_profiles: doubles.clone(),
            terminals: doubles.clone(),
            logging: doubles.clone(),
            clock: doubles.clone(),
            events: doubles.clone(),
            terminal_events: doubles.clone(),
        });
        Self {
            _directory: directory,
            database,
            operations,
            sessions,
            terminals,
            doubles,
        }
    }

    fn create_session(&self) -> SessionRecord {
        let prepared = self
            .sessions
            .prepare_creation(NewSessionRequest {
                agent_id: "codex-cli".to_string(),
                interaction_mode: "cli".to_string(),
                title: Some("Lifecycle integration".to_string()),
                workspace: NewSessionWorkspace {
                    folder: Some("D:/work/lifecycle".to_string()),
                    ..Default::default()
                },
                owner: SessionOwner::desktop(),
                activation: SessionActivation::Activate,
            })
            .expect("prepare session creation");
        self.sessions
            .execute_creation(prepared)
            .expect("create session")
    }

    fn open(&self, session_id: &str) -> AgentTerminalSession {
        self.terminals
            .open_or_attach(OpenAgentTerminalRequest {
                session_id: session_id.to_string(),
                size: AgentTerminalSize { rows: 24, cols: 80 },
            })
            .expect("open terminal")
    }
}

#[test]
fn session_terminal_lifecycle_persists_and_releases_all_state() {
    let harness = LifecycleHarness::new();
    let session = harness.create_session();
    let creation_operation = harness
        .operations
        .list()
        .expect("list creation operations")
        .into_iter()
        .next()
        .expect("creation operation");
    assert_eq!(creation_operation.status, OperationStatus::Succeeded);
    assert_eq!(
        creation_operation.related_entity_id.as_deref(),
        Some("D:/work/lifecycle")
    );
    assert_eq!(
        creation_operation
            .result
            .as_ref()
            .and_then(|result| result.get("id"))
            .and_then(serde_json::Value::as_str),
        Some(session.id())
    );

    let terminal = harness.open(session.id());
    let persisted = harness
        .sessions
        .find(session.id())
        .expect("find session")
        .expect("persisted session");
    assert_eq!(persisted.aggregate.lifecycle().as_str(), "running");
    assert_eq!(persisted.runtime_session_id.as_deref(), Some("runtime-1"));
    assert!(harness.doubles.has_live_terminal(session.id()));

    assert!(harness
        .terminals
        .stop(StopAgentTerminalRequest {
            terminal_id: terminal.terminal_id,
        })
        .expect("stop terminal"));
    let stopped = harness
        .sessions
        .find(session.id())
        .expect("find stopped session")
        .expect("persisted stopped session");
    assert_eq!(stopped.aggregate.lifecycle().as_str(), "stopped");
    assert!(!harness.doubles.has_live_terminal(session.id()));
    harness
        .sessions
        .delete(session.id())
        .expect("delete session");

    assert!(harness
        .sessions
        .find(session.id())
        .expect("find deleted session")
        .is_none());
    assert!(!harness.doubles.has_live_terminal(session.id()));
    let database_count: i64 = harness
        .database
        .connection()
        .expect("database connection")
        .query_row(
            "SELECT COUNT(*) FROM sessions WHERE id = ?1",
            [session.id()],
            |row| row.get(0),
        )
        .expect("session row count");
    assert_eq!(database_count, 0);
}

#[test]
fn startup_failure_persists_failed_state_and_releases_reserved_terminal_state() {
    let harness = LifecycleHarness::new();
    let session = harness.create_session();
    harness.doubles.fail_start.store(true, Ordering::SeqCst);

    let error = harness
        .terminals
        .open_or_attach(OpenAgentTerminalRequest {
            session_id: session.id().to_string(),
            size: AgentTerminalSize { rows: 24, cols: 80 },
        })
        .expect_err("terminal startup must fail");

    assert!(matches!(&error, AgentRuntimeApplicationError::Process(_)));
    let command_error = crate::commands::error::map_command_error(error);
    assert_eq!(command_error.message(), "launch failed: token=[REDACTED]");
    let persisted = harness
        .sessions
        .find(session.id())
        .expect("find session")
        .expect("persisted session");
    assert_eq!(persisted.aggregate.lifecycle().as_str(), "failed");
    assert!(!harness.doubles.has_live_terminal(session.id()));
    let persisted_log = harness
        .doubles
        .agent_logs()
        .into_iter()
        .find(|entry| {
            entry.category == "session.agent_terminal"
                && entry.session_id.as_deref() == Some(session.id())
        })
        .expect("associated startup diagnostic");
    assert!(persisted_log.message.contains("[REDACTED]"));
    assert!(!persisted_log.message.contains("private-startup-token"));
}

#[test]
fn repeated_stop_and_delete_cleanup_are_idempotent() {
    let harness = LifecycleHarness::new();
    let session = harness.create_session();
    let terminal = harness.open(session.id());
    let request = StopAgentTerminalRequest {
        terminal_id: terminal.terminal_id,
    };

    assert!(harness.terminals.stop(request.clone()).expect("first stop"));
    assert!(!harness.terminals.stop(request).expect("repeated stop"));
    harness
        .sessions
        .delete(session.id())
        .expect("delete session");
    assert!(!harness.doubles.has_live_terminal(session.id()));
    assert_eq!(
        harness.doubles.cleanup_calls(),
        vec![session.id().to_string()]
    );
}

#[derive(Default)]
struct LifecycleDoubles {
    terminals: Mutex<HashMap<String, AgentTerminalSession>>,
    agent_sessions: Mutex<Option<Arc<dyn AgentSessionGateway>>>,
    session_logs: Mutex<Vec<SessionApplicationLog>>,
    agent_log_entries: Mutex<Vec<AgentLog>>,
    terminal_events: Mutex<Vec<AgentTerminalEvent>>,
    cleanup_sessions: Mutex<Vec<String>>,
    fail_start: AtomicBool,
}

impl LifecycleDoubles {
    fn bind_agent_sessions(&self, sessions: Arc<dyn AgentSessionGateway>) {
        *self.agent_sessions.lock().expect("agent sessions") = Some(sessions);
    }

    fn agent_logs(&self) -> Vec<AgentLog> {
        self.agent_log_entries.lock().expect("agent logs").clone()
    }

    fn cleanup_calls(&self) -> Vec<String> {
        self.cleanup_sessions
            .lock()
            .expect("cleanup sessions")
            .clone()
    }

    fn has_live_terminal(&self, session_id: &str) -> bool {
        self.terminals
            .lock()
            .expect("terminals")
            .values()
            .any(|terminal| terminal.session_id == session_id)
    }
}

#[derive(Default)]
struct LifecycleOperationRepository {
    operations: Mutex<BTreeMap<String, OperationTask>>,
}

impl OperationRepository for LifecycleOperationRepository {
    fn insert(&self, operation: OperationTask) -> Result<(), OperationsApplicationError> {
        self.operations
            .lock()
            .expect("operations")
            .insert(operation.id.clone(), operation);
        Ok(())
    }

    fn update(
        &self,
        operation_id: &str,
        mutation: &mut dyn FnMut(&mut OperationTask),
    ) -> Result<OperationTask, OperationsApplicationError> {
        let mut operations = self.operations.lock().expect("operations");
        let operation = operations.get_mut(operation_id).ok_or_else(|| {
            OperationsApplicationError::NotFound(format!("operation not found: {operation_id}"))
        })?;
        mutation(operation);
        Ok(operation.clone())
    }

    fn get(&self, operation_id: &str) -> Result<OperationTask, OperationsApplicationError> {
        self.operations
            .lock()
            .expect("operations")
            .get(operation_id)
            .cloned()
            .ok_or_else(|| {
                OperationsApplicationError::NotFound(format!("operation not found: {operation_id}"))
            })
    }

    fn list(&self) -> Result<Vec<OperationTask>, OperationsApplicationError> {
        Ok(self
            .operations
            .lock()
            .expect("operations")
            .values()
            .cloned()
            .collect())
    }
}

struct LifecycleOperationClock;

impl OperationClock for LifecycleOperationClock {
    fn now(&self) -> String {
        "2026-07-23T00:00:00Z".to_string()
    }
}

struct LifecycleOperationIds;

impl OperationIdGenerator for LifecycleOperationIds {
    fn next_id(&self, _timestamp: &str) -> String {
        "operation-1".to_string()
    }
}

impl SessionClockPort for LifecycleDoubles {
    fn now(&self) -> String {
        "2026-07-23T00:00:00Z".to_string()
    }

    fn inactivity_cutoff(&self, _inactive_days: i64) -> Result<String, SessionsApplicationError> {
        Ok("2026-07-01T00:00:00Z".to_string())
    }

    fn usage_range_start(
        &self,
        _range: UsageStatisticsRange,
    ) -> Result<Option<String>, SessionsApplicationError> {
        Ok(None)
    }
}

impl SessionIdentityPort for LifecycleDoubles {
    fn next_session_id(&self) -> String {
        "session-lifecycle-1".to_string()
    }

    fn next_message_id(&self) -> String {
        "message-lifecycle-1".to_string()
    }

    fn next_category_id(&self) -> String {
        "category-lifecycle-1".to_string()
    }
}

impl SessionCreationContextPort for LifecycleDoubles {
    fn remote_workspace_uri(&self, _workspace: &NewRemoteWorkspace) -> Option<String> {
        None
    }

    fn ensure_agent_supports(
        &self,
        agent_id: &str,
        interaction_mode: &str,
    ) -> Result<(), SessionsApplicationError> {
        if agent_id == "codex-cli" && interaction_mode == "cli" {
            Ok(())
        } else {
            Err(SessionsApplicationError::Validation(
                "unsupported test agent".to_string(),
            ))
        }
    }

    fn ensure_worktree_compatible(
        &self,
        _remote_workspace_selected: bool,
        _worktree_enabled: bool,
    ) -> Result<(), SessionsApplicationError> {
        Ok(())
    }

    fn prepare_project(&self, path: &str) -> Result<SessionProject, SessionsApplicationError> {
        Ok(SessionProject {
            path: path.to_string(),
            is_git: false,
        })
    }

    fn normalize_remote_workspace(
        &self,
        _workspace: &NewRemoteWorkspace,
    ) -> Result<SessionRemoteWorkspace, SessionsApplicationError> {
        Err(SessionsApplicationError::Validation(
            "remote workspace unused".to_string(),
        ))
    }

    fn remember_remote_workspace(
        &self,
        _workspace: &SessionRemoteWorkspace,
    ) -> Result<(), SessionsApplicationError> {
        Ok(())
    }

    fn ensure_git_worktree_available(
        &self,
        _project: &SessionProject,
    ) -> Result<(), SessionsApplicationError> {
        Ok(())
    }

    fn create_worktree(
        &self,
        _project_path: &str,
        _name: &str,
    ) -> Result<CreatedSessionWorktree, SessionsApplicationError> {
        Err(SessionsApplicationError::Validation(
            "worktree creation unused".to_string(),
        ))
    }
}

impl SessionRuntimePort for LifecycleDoubles {
    fn stop_session_activity(&self, session_id: &str) -> Result<(), SessionsApplicationError> {
        self.cleanup_sessions
            .lock()
            .expect("cleanup sessions")
            .push(session_id.to_string());
        self.terminals
            .lock()
            .expect("terminals")
            .retain(|_, terminal| terminal.session_id != session_id);
        Ok(())
    }
}

impl SessionFileContentPort for LifecycleDoubles {
    fn read_reference_text(
        &self,
        _session_id: &str,
        _path: &str,
    ) -> Result<String, SessionsApplicationError> {
        Err(SessionsApplicationError::FileContent(
            "file access unused".to_string(),
        ))
    }

    fn write_export(
        &self,
        _destination_directory: &str,
        _filename: &str,
        _content: &str,
    ) -> Result<String, SessionsApplicationError> {
        Err(SessionsApplicationError::FileContent(
            "export unused".to_string(),
        ))
    }
}

impl SessionLoggingPort for LifecycleDoubles {
    fn write(&self, log: SessionApplicationLog) -> Result<(), SessionsApplicationError> {
        self.session_logs.lock().expect("session logs").push(log);
        Ok(())
    }
}

impl SessionChatProfilePort for LifecycleDoubles {
    fn defaults_for(
        &self,
        _agent_id: &str,
    ) -> Result<ChatConfigurationValues, SessionsApplicationError> {
        Ok(ChatConfigurationValues {
            permission_mode: "agent".to_string(),
            provider_id: Some("openai".to_string()),
            model_id: Some("gpt-test".to_string()),
            reasoning_depth: None,
            streaming: true,
            thinking: false,
            long_context: false,
        })
    }
}

impl AgentRegistryRepository for LifecycleDoubles {
    fn list(&self) -> Result<Vec<AgentDefinition>, AgentRuntimeApplicationError> {
        Ok(vec![test_agent()])
    }

    fn find(
        &self,
        agent_id: &str,
    ) -> Result<Option<AgentDefinition>, AgentRuntimeApplicationError> {
        Ok((agent_id == "codex-cli").then(test_agent))
    }
}

impl AgentCliProfileGateway for LifecycleDoubles {
    fn load(
        &self,
        _agent_id: &str,
        _configuration: &crate::contexts::agent_runtime::application::AgentChatConfiguration,
    ) -> Result<CliProfileSnapshot, AgentRuntimeApplicationError> {
        self.load_interactive("codex-cli")
    }

    fn load_interactive(
        &self,
        _agent_id: &str,
    ) -> Result<CliProfileSnapshot, AgentRuntimeApplicationError> {
        Ok(CliProfileSnapshot {
            executable: "deterministic-process-double".to_string(),
            selections: BTreeMap::new(),
            managed_args: Vec::new(),
        })
    }
}

impl AgentTerminalGateway for LifecycleDoubles {
    fn attach_retained(
        &self,
        session_id: &str,
    ) -> Result<Option<AgentTerminalSession>, AgentRuntimeApplicationError> {
        Ok(self
            .terminals
            .lock()
            .expect("terminals")
            .values()
            .find(|terminal| terminal.session_id == session_id)
            .cloned())
    }

    fn open_or_attach(
        &self,
        request: AgentTerminalProcessRequest,
    ) -> Result<AgentTerminalSession, AgentRuntimeApplicationError> {
        if self.fail_start.load(Ordering::SeqCst) {
            return Err(AgentRuntimeApplicationError::Process(
                "token=private-startup-token".to_string(),
            ));
        }
        let terminal = AgentTerminalSession {
            terminal_id: format!("terminal-{}", request.session.id),
            session_id: request.session.id,
            agent_id: request.agent.id,
            state: AgentTerminalState::Running,
            capability:
                crate::contexts::agent_runtime::application::AgentTerminalCapability::Native,
            size: request.size,
            runtime_session_id: Some("runtime-1".to_string()),
            retained: true,
        };
        self.terminals
            .lock()
            .expect("terminals")
            .insert(terminal.terminal_id.clone(), terminal.clone());
        Ok(terminal)
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
        let terminal = self
            .terminals
            .lock()
            .expect("terminals")
            .remove(&request.terminal_id);
        let Some(terminal) = terminal else {
            return Ok(false);
        };
        let sessions = self
            .agent_sessions
            .lock()
            .expect("agent sessions")
            .clone()
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Session(
                    "test Agent Session gateway is unavailable".to_string(),
                )
            })?;
        sessions.update_lifecycle(&terminal.session_id, AgentLifecycle::Stopped)?;
        Ok(true)
    }

    fn cleanup_idle(
        &self,
        _idle_after_seconds: i64,
    ) -> Result<Vec<String>, AgentRuntimeApplicationError> {
        Ok(Vec::new())
    }

    fn shutdown(&self) -> Result<Vec<String>, AgentRuntimeApplicationError> {
        Ok(Vec::new())
    }
}

impl AgentLoggingPort for LifecycleDoubles {
    fn record(&self, mut log: AgentLog) -> Result<(), AgentRuntimeApplicationError> {
        log.message = crate::platform::logging::redact_text(&log.message);
        self.agent_log_entries.lock().expect("agent logs").push(log);
        Ok(())
    }
}

impl AgentClockPort for LifecycleDoubles {
    fn now(&self) -> String {
        "2026-07-23T00:00:00Z".to_string()
    }
}

impl AgentEventPort for LifecycleDoubles {
    fn publish(&self, _event: AgentEvent) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
}

impl AgentTerminalEventPort for LifecycleDoubles {
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

fn test_agent() -> AgentDefinition {
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
        .expect("launch metadata"),
        supported_interaction_modes: vec![InteractionMode::Cli],
        availability: AvailabilityAssessment::new(AgentAvailability::Available, None),
        capability_tags: vec!["coding".to_string()],
    })
    .expect("test agent")
}
