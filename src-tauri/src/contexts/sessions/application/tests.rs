use super::*;
use crate::contexts::sessions::domain::{
    CategoryId, CategoryName, FileReferenceSet, LoopSessionRole, MessageId, MessageRole,
    MessageStatus, SessionActivation, SessionAggregate, SessionCategory, SessionId,
    SessionLifecycle, SessionMessage, SessionOwner, SessionTitle,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct FakeStore {
    sessions: Mutex<BTreeMap<String, SessionRecord>>,
    active_session_id: Mutex<Option<String>>,
    messages: Mutex<BTreeMap<String, MessageRecord>>,
    categories: Mutex<BTreeMap<String, CategoryRecord>>,
    configurations: Mutex<BTreeMap<String, ChatConfigurationValues>>,
    inactive_session_ids: Mutex<BTreeSet<String>>,
    events: Mutex<Vec<String>>,
    search_queries: Mutex<Vec<SessionSearchQuery>>,
    message_queries: Mutex<Vec<MessagePageQuery>>,
    usage_queries: Mutex<Vec<(UsageStatisticsRange, Option<String>, String)>>,
    fail_create: AtomicBool,
}

impl FakeStore {
    fn seed_session(&self, session: SessionRecord) {
        self.sessions
            .lock()
            .expect("sessions")
            .insert(session.id().to_string(), session);
    }

    fn seed_message(&self, message: MessageRecord) {
        self.messages
            .lock()
            .expect("messages")
            .insert(message.message.id().as_str().to_string(), message);
    }
}

impl SessionRepository for FakeStore {
    fn find(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<SessionRecord>, SessionsApplicationError> {
        Ok(self
            .sessions
            .lock()
            .expect("sessions")
            .get(session_id.as_str())
            .cloned())
    }

    fn list(
        &self,
        scope: SessionListScope,
    ) -> Result<Vec<SessionRecord>, SessionsApplicationError> {
        Ok(self
            .sessions
            .lock()
            .expect("sessions")
            .values()
            .filter(|session| {
                session.aggregate.is_archived() == (scope == SessionListScope::Archived)
                    && session.workspace.loop_ownership.is_none()
            })
            .cloned()
            .collect())
    }

    #[cfg(test)]
    fn list_including_loop_owned(
        &self,
        scope: SessionListScope,
    ) -> Result<Vec<SessionRecord>, SessionsApplicationError> {
        Ok(self
            .sessions
            .lock()
            .expect("sessions")
            .values()
            .filter(|session| {
                session.aggregate.is_archived() == (scope == SessionListScope::Archived)
            })
            .cloned()
            .collect())
    }

    fn search(
        &self,
        query: &SessionSearchQuery,
    ) -> Result<Vec<SessionSearchResult>, SessionsApplicationError> {
        self.search_queries
            .lock()
            .expect("search queries")
            .push(query.clone());
        Ok(self
            .sessions
            .lock()
            .expect("sessions")
            .values()
            .filter(|session| session.workspace.loop_ownership.is_none())
            .take(query.limit)
            .cloned()
            .map(|session| SessionSearchResult {
                matches: vec![SessionSearchMatch {
                    kind: SessionSearchMatchKind::Title,
                    excerpt: session.aggregate.title().as_str().to_string(),
                    message_id: None,
                }],
                session,
            })
            .collect())
    }

    fn active_session(&self) -> Result<Option<SessionRecord>, SessionsApplicationError> {
        let active = self
            .active_session_id
            .lock()
            .expect("active session")
            .clone();
        Ok(active.and_then(|session_id| {
            self.sessions
                .lock()
                .expect("sessions")
                .get(&session_id)
                .cloned()
        }))
    }

    fn save(&self, session: &SessionRecord) -> Result<SessionRecord, SessionsApplicationError> {
        self.sessions
            .lock()
            .expect("sessions")
            .insert(session.id().to_string(), session.clone());
        self.events
            .lock()
            .expect("events")
            .push(format!("save:{}", session.id()));
        Ok(session.clone())
    }

    fn recoverable_sessions(&self) -> Result<Vec<SessionRecord>, SessionsApplicationError> {
        Ok(self
            .sessions
            .lock()
            .expect("sessions")
            .values()
            .filter(|session| session.aggregate.lifecycle().has_active_generation())
            .cloned()
            .collect())
    }

    fn inactive_sessions(
        &self,
        _cutoff: &str,
    ) -> Result<Vec<SessionRecord>, SessionsApplicationError> {
        let ids = self
            .inactive_session_ids
            .lock()
            .expect("inactive session ids")
            .clone();
        Ok(self
            .sessions
            .lock()
            .expect("sessions")
            .iter()
            .filter(|(session_id, _)| ids.contains(*session_id))
            .map(|(_, session)| session.clone())
            .collect())
    }
}

impl SessionMessageRepository for FakeStore {
    fn find(
        &self,
        message_id: &MessageId,
    ) -> Result<Option<MessageRecord>, SessionsApplicationError> {
        Ok(self
            .messages
            .lock()
            .expect("messages")
            .get(message_id.as_str())
            .cloned())
    }

    fn insert(&self, message: &MessageRecord) -> Result<MessageRecord, SessionsApplicationError> {
        self.seed_message(message.clone());
        self.events
            .lock()
            .expect("events")
            .push(format!("insert-message:{}", message.message.id().as_str()));
        Ok(message.clone())
    }

    fn save(&self, message: &MessageRecord) -> Result<MessageRecord, SessionsApplicationError> {
        self.seed_message(message.clone());
        self.events
            .lock()
            .expect("events")
            .push(format!("save-message:{}", message.message.id().as_str()));
        Ok(message.clone())
    }

    fn save_stream_fields(&self, message: &MessageRecord) -> Result<(), SessionsApplicationError> {
        let mut messages = self.messages.lock().expect("messages");
        let current = messages
            .get_mut(message.message.id().as_str())
            .ok_or_else(|| {
                SessionsApplicationError::MessageNotFound(message.message.id().as_str().to_string())
            })?;
        current.content.clone_from(&message.content);
        current
            .thinking_content
            .clone_from(&message.thinking_content);
        current.tool_use.clone_from(&message.tool_use);
        current.rich_blocks.clone_from(&message.rich_blocks);
        current.updated_at.clone_from(&message.updated_at);
        Ok(())
    }

    fn list(
        &self,
        query: &MessagePageQuery,
    ) -> Result<Vec<MessageRecord>, SessionsApplicationError> {
        self.message_queries
            .lock()
            .expect("message queries")
            .push(query.clone());
        Ok(self
            .messages
            .lock()
            .expect("messages")
            .values()
            .filter(|message| message.message.session_id().as_str() == query.session_id)
            .take(query.limit)
            .cloned()
            .collect())
    }

    fn list_all(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<MessageRecord>, SessionsApplicationError> {
        SessionMessageRepository::list(
            self,
            &MessagePageQuery {
                session_id: session_id.as_str().to_string(),
                limit: usize::MAX,
                before_id: None,
            },
        )
    }
}

impl SessionCategoryRepository for FakeStore {
    fn list(&self) -> Result<Vec<CategoryRecord>, SessionsApplicationError> {
        Ok(self
            .categories
            .lock()
            .expect("categories")
            .values()
            .cloned()
            .collect())
    }

    fn find(
        &self,
        category_id: &CategoryId,
    ) -> Result<Option<CategoryRecord>, SessionsApplicationError> {
        Ok(self
            .categories
            .lock()
            .expect("categories")
            .get(category_id.as_str())
            .cloned())
    }

    fn name_exists(
        &self,
        name: &str,
        excluding: Option<&CategoryId>,
    ) -> Result<bool, SessionsApplicationError> {
        Ok(self
            .categories
            .lock()
            .expect("categories")
            .values()
            .any(|record| {
                record.category.name().as_str().eq_ignore_ascii_case(name)
                    && excluding.is_none_or(|id| id != record.category.id())
            }))
    }

    fn next_sort_order(&self) -> Result<i64, SessionsApplicationError> {
        Ok(self.categories.lock().expect("categories").len() as i64)
    }

    fn insert(
        &self,
        category: &CategoryRecord,
    ) -> Result<CategoryRecord, SessionsApplicationError> {
        self.categories.lock().expect("categories").insert(
            category.category.id().as_str().to_string(),
            category.clone(),
        );
        Ok(category.clone())
    }

    fn save(&self, category: &CategoryRecord) -> Result<CategoryRecord, SessionsApplicationError> {
        SessionCategoryRepository::insert(self, category)
    }
}

impl SessionConfigurationRepository for FakeStore {
    fn load(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<ChatConfigurationValues>, SessionsApplicationError> {
        Ok(self
            .configurations
            .lock()
            .expect("configurations")
            .get(session_id.as_str())
            .cloned())
    }

    fn save(
        &self,
        session_id: &SessionId,
        preferences: &crate::contexts::sessions::domain::ChatPreferences,
        _updated_at: &str,
    ) -> Result<(), SessionsApplicationError> {
        self.configurations.lock().expect("configurations").insert(
            session_id.as_str().to_string(),
            ChatConfigurationValues::from_preferences(preferences),
        );
        Ok(())
    }
}

impl SessionUsageRepository for FakeStore {
    fn statistics(
        &self,
        range: UsageStatisticsRange,
        range_start: Option<&str>,
        generated_at: &str,
    ) -> Result<SessionUsageStatistics, SessionsApplicationError> {
        self.usage_queries.lock().expect("usage queries").push((
            range,
            range_start.map(str::to_string),
            generated_at.to_string(),
        ));
        Ok(SessionUsageStatistics {
            range,
            reported: ReportedTokenTotals {
                input_tokens: 3,
                output_tokens: 5,
                total_tokens: 8,
                ..Default::default()
            },
            estimated: EstimatedCharacterTotals::default(),
            coverage: SessionUsageCoverage {
                reported_responses: 1,
                total_responses: 1,
                reported_percent: 100.0,
                ..Default::default()
            },
            counted_sessions: 1,
            daily: Vec::new(),
            by_agent: Vec::new(),
            generated_at: generated_at.to_string(),
        })
    }

    fn summary_for_session(
        &self,
        session_id: &str,
        generated_at: &str,
    ) -> Result<SessionUsageSummary, SessionsApplicationError> {
        self.usage_queries.lock().expect("usage queries").push((
            UsageStatisticsRange::All,
            Some(session_id.to_string()),
            generated_at.to_string(),
        ));
        Ok(SessionUsageSummary {
            session_id: session_id.to_string(),
            reported: ReportedTokenTotals {
                input_tokens: 3,
                output_tokens: 5,
                total_tokens: 8,
                ..Default::default()
            },
            estimated: EstimatedCharacterTotals::default(),
            coverage: SessionUsageCoverage {
                reported_responses: 1,
                total_responses: 1,
                reported_percent: 100.0,
                ..Default::default()
            },
            response_count: 1,
            generated_at: generated_at.to_string(),
        })
    }
}

impl SessionTransactionPort for FakeStore {
    fn create_session(
        &self,
        session: &SessionRecord,
        activation: SessionActivation,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        if self.fail_create.load(Ordering::SeqCst) {
            return Err(SessionsApplicationError::Transaction(
                "injected create failure".to_string(),
            ));
        }
        self.seed_session(session.clone());
        if activation == SessionActivation::Activate {
            *self.active_session_id.lock().expect("active session id") =
                Some(session.id().to_string());
        }
        self.events
            .lock()
            .expect("events")
            .push(format!("create:{}:{activation:?}", session.id()));
        Ok(session.clone())
    }

    fn activate_session(
        &self,
        session: &SessionRecord,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        *self.active_session_id.lock().expect("active session id") = Some(session.id().to_string());
        self.events
            .lock()
            .expect("events")
            .push(format!("activate:{}", session.id()));
        Ok(session.clone())
    }

    fn archive_session(
        &self,
        session: &SessionRecord,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        self.seed_session(session.clone());
        self.clear_active_session_if_matches(session.aggregate.id())?;
        self.events
            .lock()
            .expect("events")
            .push(format!("archive:{}", session.id()));
        Ok(session.clone())
    }

    fn clear_active_session_if_matches(
        &self,
        session_id: &SessionId,
    ) -> Result<(), SessionsApplicationError> {
        let mut active = self.active_session_id.lock().expect("active session id");
        if active.as_deref() == Some(session_id.as_str()) {
            *active = None;
        }
        Ok(())
    }

    fn delete_session(&self, session_id: &SessionId) -> Result<(), SessionsApplicationError> {
        self.sessions
            .lock()
            .expect("sessions")
            .remove(session_id.as_str());
        self.messages
            .lock()
            .expect("messages")
            .retain(|_, message| message.message.session_id() != session_id);
        self.clear_active_session_if_matches(session_id)?;
        self.events
            .lock()
            .expect("events")
            .push(format!("delete:{}", session_id.as_str()));
        Ok(())
    }

    fn delete_category(
        &self,
        category_id: &CategoryId,
        updated_at: &str,
    ) -> Result<(), SessionsApplicationError> {
        self.categories
            .lock()
            .expect("categories")
            .remove(category_id.as_str());
        for session in self.sessions.lock().expect("sessions").values_mut() {
            if session.aggregate.category_id() == Some(category_id) {
                session.aggregate.assign_category(None);
                session.updated_at = updated_at.to_string();
            }
        }
        self.events
            .lock()
            .expect("events")
            .push(format!("delete-category:{}", category_id.as_str()));
        Ok(())
    }

    fn complete_message(
        &self,
        message: &MessageRecord,
        usage: Option<&MessageUsageRecord>,
    ) -> Result<MessageRecord, SessionsApplicationError> {
        self.seed_message(message.clone());
        self.events.lock().expect("events").push(format!(
            "complete-message:{}:usage={}",
            message.message.id().as_str(),
            usage.is_some()
        ));
        Ok(message.clone())
    }

    fn save_runtime_session(
        &self,
        session: &SessionRecord,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        self.seed_session(session.clone());
        self.events
            .lock()
            .expect("events")
            .push(format!("runtime-session:{}", session.id()));
        Ok(session.clone())
    }

    fn cancel_messages(
        &self,
        messages: &[MessageRecord],
    ) -> Result<Vec<String>, SessionsApplicationError> {
        for message in messages {
            self.seed_message(message.clone());
        }
        self.events
            .lock()
            .expect("events")
            .push(format!("cancel-messages:{}", messages.len()));
        Ok(messages
            .iter()
            .map(|message| message.message.id().as_str().to_string())
            .collect())
    }

    fn recover_orphaned_session(
        &self,
        session: &SessionRecord,
        _recovered_at: &str,
    ) -> Result<(), SessionsApplicationError> {
        self.seed_session(session.clone());
        self.events
            .lock()
            .expect("events")
            .push(format!("recover:{}", session.id()));
        Ok(())
    }
}

struct FakeClock {
    calls: Mutex<Vec<String>>,
}

impl Default for FakeClock {
    fn default() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
        }
    }
}

impl SessionClockPort for FakeClock {
    fn now(&self) -> String {
        "2026-07-18T10:00:00+00:00".to_string()
    }

    fn inactivity_cutoff(&self, inactive_days: i64) -> Result<String, SessionsApplicationError> {
        self.calls
            .lock()
            .expect("clock calls")
            .push(format!("cutoff:{inactive_days}"));
        Ok("2026-07-08T10:00:00+00:00".to_string())
    }

    fn usage_range_start(
        &self,
        range: UsageStatisticsRange,
    ) -> Result<Option<String>, SessionsApplicationError> {
        self.calls
            .lock()
            .expect("clock calls")
            .push(format!("usage:{range:?}"));
        Ok((range != UsageStatisticsRange::All).then(|| "2026-07-12T16:00:00+00:00".to_string()))
    }
}

struct FakeIdentities;

impl SessionIdentityPort for FakeIdentities {
    fn next_session_id(&self) -> String {
        "session-created".to_string()
    }

    fn next_message_id(&self) -> String {
        "message-created".to_string()
    }

    fn next_category_id(&self) -> String {
        "category-created".to_string()
    }
}

#[derive(Default)]
struct FakeFiles {
    contents: Mutex<BTreeMap<String, String>>,
    writes: Mutex<Vec<(String, String, String)>>,
}

impl SessionFileContentPort for FakeFiles {
    fn read_reference_text(
        &self,
        _session_id: &str,
        path: &str,
    ) -> Result<String, SessionsApplicationError> {
        self.contents
            .lock()
            .expect("file contents")
            .get(path)
            .cloned()
            .ok_or_else(|| SessionsApplicationError::FileContent("missing fixture".to_string()))
    }

    fn write_export(
        &self,
        destination_directory: &str,
        filename: &str,
        content: &str,
    ) -> Result<String, SessionsApplicationError> {
        self.writes.lock().expect("writes").push((
            destination_directory.to_string(),
            filename.to_string(),
            content.to_string(),
        ));
        Ok(format!("{destination_directory}\\{filename}"))
    }
}

#[derive(Default)]
struct FakeOperations {
    events: Mutex<Vec<String>>,
}

impl SessionOperationPort for FakeOperations {
    fn start_session_creation(
        &self,
        related_entity_id: Option<String>,
    ) -> Result<SessionCreationOperation, SessionsApplicationError> {
        self.events
            .lock()
            .expect("operation events")
            .push(format!("start:{related_entity_id:?}"));
        Ok(SessionCreationOperation {
            id: "operation-session-1".to_string(),
            related_entity_id,
            message: Some("Creating session".to_string()),
            created_at: "100".to_string(),
            updated_at: "100".to_string(),
        })
    }

    fn append_log(&self, operation_id: &str, line: String) -> Result<(), SessionsApplicationError> {
        self.events
            .lock()
            .expect("operation events")
            .push(format!("log:{operation_id}:{line}"));
        Ok(())
    }

    fn complete_session_creation(
        &self,
        operation_id: &str,
        session: &SessionRecord,
    ) -> Result<(), SessionsApplicationError> {
        self.events
            .lock()
            .expect("operation events")
            .push(format!("complete:{operation_id}:{}", session.id()));
        Ok(())
    }

    fn fail_session_creation(
        &self,
        operation_id: &str,
        error: String,
    ) -> Result<(), SessionsApplicationError> {
        self.events
            .lock()
            .expect("operation events")
            .push(format!("fail:{operation_id}:{error}"));
        Ok(())
    }
}

#[derive(Default)]
struct FakeLogging {
    entries: Mutex<Vec<SessionApplicationLog>>,
}

impl SessionLoggingPort for FakeLogging {
    fn write(&self, log: SessionApplicationLog) -> Result<(), SessionsApplicationError> {
        self.entries.lock().expect("log entries").push(log);
        Ok(())
    }
}

struct FakeChatProfiles;

impl SessionChatProfilePort for FakeChatProfiles {
    fn defaults_for(
        &self,
        agent_id: &str,
    ) -> Result<ChatConfigurationValues, SessionsApplicationError> {
        let (provider_id, model_id) = match agent_id {
            "gemini-cli" => ("google", "gemini-2-5-flash"),
            "codex-cli" => ("openai", "gpt-5-5"),
            _ => ("anthropic", "claude-opus-4-8"),
        };
        Ok(ChatConfigurationValues {
            permission_mode: "agent".to_string(),
            provider_id: Some(provider_id.to_string()),
            model_id: Some(model_id.to_string()),
            reasoning_depth: Some("max".to_string()),
            streaming: true,
            thinking: true,
            long_context: false,
        })
    }
}

#[derive(Default)]
struct FakeCreationContext {
    events: Mutex<Vec<String>>,
}

impl SessionCreationContextPort for FakeCreationContext {
    fn remote_workspace_uri(&self, workspace: &NewRemoteWorkspace) -> Option<String> {
        Some(format!(
            "ssh://{}{}{}{}",
            workspace
                .user
                .as_deref()
                .map(|user| format!("{user}@"))
                .unwrap_or_default(),
            workspace.host,
            workspace
                .port
                .filter(|port| *port != 22)
                .map(|port| format!(":{port}"))
                .unwrap_or_default(),
            workspace.path
        ))
    }

    fn ensure_agent_supports(
        &self,
        agent_id: &str,
        interaction_mode: &str,
    ) -> Result<(), SessionsApplicationError> {
        self.events
            .lock()
            .expect("creation events")
            .push(format!("agent:{agent_id}:{interaction_mode}"));
        Ok(())
    }

    fn ensure_worktree_compatible(
        &self,
        remote_workspace_selected: bool,
        worktree_enabled: bool,
    ) -> Result<(), SessionsApplicationError> {
        self.events.lock().expect("creation events").push(format!(
            "compatible:{remote_workspace_selected}:{worktree_enabled}"
        ));
        Ok(())
    }

    fn prepare_project(&self, path: &str) -> Result<SessionProject, SessionsApplicationError> {
        self.events
            .lock()
            .expect("creation events")
            .push(format!("project:{path}"));
        Ok(SessionProject {
            path: path.to_string(),
            is_git: true,
        })
    }

    fn normalize_remote_workspace(
        &self,
        workspace: &NewRemoteWorkspace,
    ) -> Result<SessionRemoteWorkspace, SessionsApplicationError> {
        let uri = self
            .remote_workspace_uri(workspace)
            .expect("remote workspace uri");
        Ok(SessionRemoteWorkspace {
            host: workspace.host.clone(),
            port: workspace.port,
            user: workspace.user.clone(),
            path: workspace.path.clone(),
            display_name: workspace
                .display_name
                .clone()
                .unwrap_or_else(|| workspace.host.clone()),
            uri,
        })
    }

    fn remember_remote_workspace(
        &self,
        workspace: &SessionRemoteWorkspace,
    ) -> Result<(), SessionsApplicationError> {
        self.events
            .lock()
            .expect("creation events")
            .push(format!("remote:{}", workspace.uri));
        Ok(())
    }

    fn ensure_git_worktree_available(
        &self,
        project: &SessionProject,
    ) -> Result<(), SessionsApplicationError> {
        self.events
            .lock()
            .expect("creation events")
            .push(format!("git:{}", project.path));
        Ok(())
    }

    fn create_worktree(
        &self,
        project_path: &str,
        name: &str,
    ) -> Result<CreatedSessionWorktree, SessionsApplicationError> {
        self.events
            .lock()
            .expect("creation events")
            .push(format!("worktree:{project_path}:{name}"));
        Ok(CreatedSessionWorktree {
            path: format!("{project_path}\\.worktrees\\{name}"),
            name: name.to_string(),
            branch: format!("vanehub/{name}"),
        })
    }
}

#[derive(Default)]
struct FakeRuntime {
    events: Mutex<Vec<String>>,
}

impl SessionRuntimePort for FakeRuntime {
    fn stop_session_activity(&self, session_id: &str) -> Result<(), SessionsApplicationError> {
        self.events
            .lock()
            .expect("runtime events")
            .push(format!("stop:{session_id}"));
        Ok(())
    }
}

struct Fixture {
    service: SessionsApplicationService,
    store: Arc<FakeStore>,
    clock: Arc<FakeClock>,
    files: Arc<FakeFiles>,
    operations: Arc<FakeOperations>,
    logging: Arc<FakeLogging>,
    creation: Arc<FakeCreationContext>,
    runtime: Arc<FakeRuntime>,
}

fn fixture() -> Fixture {
    let store = Arc::new(FakeStore::default());
    let clock = Arc::new(FakeClock::default());
    let files = Arc::new(FakeFiles::default());
    let operations = Arc::new(FakeOperations::default());
    let logging = Arc::new(FakeLogging::default());
    let creation = Arc::new(FakeCreationContext::default());
    let runtime = Arc::new(FakeRuntime::default());
    let service = SessionsApplicationService::new(SessionApplicationPorts {
        sessions: store.clone(),
        messages: store.clone(),
        categories: store.clone(),
        configurations: store.clone(),
        usage: store.clone(),
        transactions: store.clone(),
        clock: clock.clone(),
        identities: Arc::new(FakeIdentities),
        files: files.clone(),
        operations: operations.clone(),
        logging: logging.clone(),
        chat_profiles: Arc::new(FakeChatProfiles),
        creation: creation.clone(),
        runtime: runtime.clone(),
    });
    Fixture {
        service,
        store,
        clock,
        files,
        operations,
        logging,
        creation,
        runtime,
    }
}

fn session_record(
    id: &str,
    agent_id: &str,
    lifecycle: SessionLifecycle,
    pinned: bool,
) -> SessionRecord {
    SessionRecord {
        aggregate: SessionAggregate::rehydrate(
            SessionId::parse(id).expect("session id"),
            SessionTitle::for_creation(Some("Fixture Session")),
            lifecycle,
            SessionOwner::desktop(),
            None,
            pinned,
            false,
        ),
        agent_id: agent_id.to_string(),
        interaction_mode: "interactive".to_string(),
        workspace: SessionWorkspace {
            folder: Some("D:\\code\\fixture".to_string()),
            project_path: Some("D:\\code\\fixture".to_string()),
            ..Default::default()
        },
        runtime_session_id: None,
        created_at: "2026-07-01T00:00:00+00:00".to_string(),
        updated_at: "2026-07-01T00:00:00+00:00".to_string(),
    }
}

fn message_record(
    id: &str,
    session_id: &str,
    role: MessageRole,
    status: MessageStatus,
) -> MessageRecord {
    MessageRecord {
        message: SessionMessage::rehydrate(
            MessageId::parse(id).expect("message id"),
            SessionId::parse(session_id).expect("session id"),
            role,
            status,
            FileReferenceSet::default(),
        ),
        content: String::new(),
        thinking_content: None,
        tool_use: None,
        rich_blocks: None,
        token_usage: None,
        error: None,
        created_at: "2026-07-18T10:00:00+00:00".to_string(),
        updated_at: "2026-07-18T10:00:00+00:00".to_string(),
    }
}

fn reference(path: &str) -> FileReferenceInput {
    FileReferenceInput {
        id: format!("reference-{path}"),
        path: path.to_string(),
        name: path.to_string(),
        size_bytes: Some(12),
        content_hash: Some("hash".to_string()),
    }
}

#[test]
fn loop_role_sessions_preserve_active_session_and_are_hidden_by_default() {
    let fixture = fixture();
    let active = session_record("session-active", "codex-cli", SessionLifecycle::Idle, false);
    fixture.store.seed_session(active.clone());
    *fixture
        .store
        .active_session_id
        .lock()
        .expect("active session") = Some(active.id().to_string());

    let role_session = fixture
        .service
        .create_loop_role_session(LoopRoleSessionRequest {
            run_id: "run-1".to_string(),
            iteration_id: "iteration-1".to_string(),
            role: LoopSessionRole::Verifier,
            agent_id: "claude-code".to_string(),
            interaction_mode: "interactive".to_string(),
            project_path: "D:\\code\\project".to_string(),
            worktree_path: "D:\\code\\project-loop-1".to_string(),
            worktree_name: "loop-1".to_string(),
            worktree_branch: "vanehub/loop-1".to_string(),
        })
        .expect("role session");

    assert_eq!(
        fixture
            .store
            .active_session_id
            .lock()
            .expect("active session")
            .as_deref(),
        Some("session-active")
    );
    let ownership = role_session
        .workspace
        .loop_ownership
        .as_ref()
        .expect("Loop ownership");
    assert_eq!(ownership.run_id, "run-1");
    assert_eq!(ownership.iteration_id, "iteration-1");
    assert_eq!(ownership.role, LoopSessionRole::Verifier);
    assert_eq!(
        role_session.workspace.folder.as_deref(),
        Some("D:\\code\\project-loop-1")
    );
    assert_eq!(
        fixture
            .service
            .list_sessions(SessionListScope::Current)
            .expect("normal sessions")
            .len(),
        1
    );
    assert_eq!(
        fixture
            .service
            .list_sessions_including_loop_owned(SessionListScope::Current)
            .expect("all sessions")
            .len(),
        2
    );
    assert!(fixture
        .service
        .find_session(role_session.id())
        .expect("find role session")
        .is_some());
    assert_eq!(
        fixture
            .store
            .events
            .lock()
            .expect("events")
            .last()
            .map(String::as_str),
        Some("create:session-created:PreserveActive")
    );
}

#[test]
fn creation_management_and_category_use_cases_keep_atomic_boundaries() {
    let fixture = fixture();
    let prepared = fixture
        .service
        .prepare_new_session_creation(NewSessionRequest {
            agent_id: "codex-cli".to_string(),
            interaction_mode: "interactive".to_string(),
            title: Some("  New Session  ".to_string()),
            workspace: NewSessionWorkspace {
                project_path: Some("D:\\code\\project".to_string()),
                ..Default::default()
            },
            owner: SessionOwner::desktop(),
            activation: SessionActivation::Activate,
        })
        .expect("prepare creation");
    assert_eq!(prepared.operation.id, "operation-session-1");
    let session = fixture
        .service
        .execute_new_session_creation(prepared)
        .expect("execute creation");

    assert_eq!(session.id(), "session-created");
    assert_eq!(session.aggregate.title().as_str(), "New Session");
    assert_eq!(
        fixture
            .store
            .active_session_id
            .lock()
            .expect("active session")
            .as_deref(),
        Some("session-created")
    );
    assert!(fixture
        .operations
        .events
        .lock()
        .expect("operation events")
        .iter()
        .any(|event| event == "complete:operation-session-1:session-created"));

    let category = fixture
        .service
        .create_category("  Work  ".to_string())
        .expect("create category");
    let assigned = fixture
        .service
        .assign_category(session.id(), Some(category.category.id().as_str()))
        .expect("assign category");
    assert_eq!(
        assigned.aggregate.category_id(),
        Some(category.category.id())
    );
    let renamed = fixture
        .service
        .rename_session(session.id(), "Renamed".to_string())
        .expect("rename session");
    assert_eq!(renamed.aggregate.title().as_str(), "Renamed");
    fixture
        .service
        .set_session_pinned(session.id(), true)
        .expect("pin session");
    let archived = fixture
        .service
        .set_session_archived(session.id(), true)
        .expect("archive session");
    assert!(archived.aggregate.is_archived());
    assert!(fixture
        .store
        .active_session_id
        .lock()
        .expect("active session")
        .is_none());
    fixture
        .service
        .delete_category(category.category.id().as_str())
        .expect("delete category");
    fixture
        .service
        .delete_session(session.id())
        .expect("delete session");
    assert_eq!(
        *fixture.runtime.events.lock().expect("runtime events"),
        vec![
            "stop:session-created".to_string(),
            "stop:session-created".to_string()
        ]
    );
    assert!(fixture
        .store
        .events
        .lock()
        .expect("events")
        .iter()
        .any(|event| event == "delete:session-created"));
}

#[test]
fn raw_creation_request_prepares_project_and_worktree_before_persistence() {
    let fixture = fixture();
    let prepared = fixture
        .service
        .prepare_new_session_creation(NewSessionRequest {
            agent_id: "codex-cli".to_string(),
            interaction_mode: "cli".to_string(),
            title: Some("Worktree Session".to_string()),
            workspace: NewSessionWorkspace {
                project_path: Some("D:\\code\\project".to_string()),
                worktree: Some(NewWorktree {
                    enabled: true,
                    name: Some("feature-one".to_string()),
                }),
                ..Default::default()
            },
            owner: SessionOwner::desktop(),
            activation: SessionActivation::Activate,
        })
        .expect("prepare creation");

    let session = fixture
        .service
        .execute_new_session_creation(prepared)
        .expect("execute creation");

    assert_eq!(
        session.workspace.folder.as_deref(),
        Some("D:\\code\\project\\.worktrees\\feature-one")
    );
    assert_eq!(
        session.workspace.worktree_name.as_deref(),
        Some("feature-one")
    );
    assert_eq!(
        *fixture.creation.events.lock().expect("creation events"),
        vec![
            "agent:codex-cli:cli".to_string(),
            "compatible:false:true".to_string(),
            "project:D:\\code\\project".to_string(),
            "git:D:\\code\\project".to_string(),
            "worktree:D:\\code\\project:feature-one".to_string(),
        ]
    );
}

#[test]
fn failed_creation_records_one_operation_failure_and_diagnostic() {
    let fixture = fixture();
    fixture.store.fail_create.store(true, Ordering::SeqCst);
    let prepared = fixture
        .service
        .prepare_new_session_creation(NewSessionRequest {
            agent_id: "codex-cli".to_string(),
            interaction_mode: "interactive".to_string(),
            title: None,
            workspace: NewSessionWorkspace::default(),
            owner: SessionOwner::desktop(),
            activation: SessionActivation::Activate,
        })
        .expect("prepare creation");

    assert!(fixture
        .service
        .execute_new_session_creation(prepared)
        .is_err());
    assert!(fixture
        .operations
        .events
        .lock()
        .expect("operation events")
        .iter()
        .any(|event| event.starts_with("fail:operation-session-1:")));
    assert_eq!(
        fixture.logging.entries.lock().expect("log entries")[0].category,
        "session.create"
    );
}

#[test]
fn configuration_message_file_and_export_use_cases_use_only_ports() {
    let fixture = fixture();
    let session = session_record(
        "session-fixture",
        "gemini-cli",
        SessionLifecycle::Idle,
        false,
    );
    fixture.store.seed_session(session.clone());
    fixture
        .store
        .configurations
        .lock()
        .expect("configurations")
        .insert(
            session.id().to_string(),
            ChatConfigurationValues {
                permission_mode: "invalid".to_string(),
                provider_id: Some("openai".to_string()),
                model_id: Some("gpt-5-5".to_string()),
                reasoning_depth: None,
                streaming: false,
                thinking: false,
                long_context: false,
            },
        );
    let mut defaults = fixture
        .service
        .load_chat_configuration(session.id())
        .expect("load defaults");
    assert_eq!(defaults.values.provider_id.as_deref(), Some("google"));
    assert_eq!(defaults.values.reasoning_depth.as_deref(), Some("medium"));
    defaults.agent_id = "ignored-agent".to_string();
    defaults.interaction_mode = "ignored-mode".to_string();
    let validated = fixture
        .service
        .validate_chat_configuration(defaults.clone())
        .expect("validate configuration");
    assert_eq!(validated.agent_id, "gemini-cli");
    assert_eq!(validated.interaction_mode, "interactive");
    let saved = fixture
        .service
        .save_chat_configuration(defaults)
        .expect("save configuration");
    assert_eq!(saved.agent_id, "gemini-cli");
    assert_eq!(saved.interaction_mode, "interactive");
    assert_eq!(saved.values.model_id.as_deref(), Some("gemini-2-5-flash"));

    fixture
        .files
        .contents
        .lock()
        .expect("file contents")
        .insert("src/main.rs".to_string(), "fn main() {}".to_string());
    let user = fixture
        .service
        .create_message(CreateMessageRequest {
            session_id: session.id().to_string(),
            role: "user".to_string(),
            status: "completed".to_string(),
            content: "  explain this  ".to_string(),
            file_references: vec![reference("src/main.rs")],
        })
        .expect("create user message");
    assert_eq!(user.content, "explain this");
    let prompt = fixture
        .service
        .compose_prompt(session.id(), &user.content, vec![reference("src/main.rs")])
        .expect("compose prompt");
    assert!(prompt.contains("--- FILE: src/main.rs ---\nfn main() {}"));

    let assistant = message_record(
        "assistant-1",
        session.id(),
        MessageRole::Assistant,
        MessageStatus::Streaming,
    );
    fixture.store.seed_message(assistant);
    let completed = fixture
        .service
        .complete_message(CompleteMessageRequest {
            message_id: "assistant-1".to_string(),
            session_id: session.id().to_string(),
            content: "done".to_string(),
            thinking_content: Some("reasoning".to_string()),
            tool_use: None,
            rich_blocks: None,
            token_usage: Some(MessageTokenUsage {
                input: 3,
                output: 5,
            }),
            usage: Some(MessageUsageRecord {
                message_id: "assistant-1".to_string(),
                session_id: session.id().to_string(),
                agent_id: "gemini-cli".to_string(),
                provider_id: Some("google".to_string()),
                model_id: Some("gemini-2-5-flash".to_string()),
                accounting_kind: SessionUsageAccountingKind::Reported,
                unit: SessionUsageUnit::Tokens,
                input_count: 3,
                output_count: 5,
                cache_read_count: 0,
                cache_creation_count: 0,
                source: "provider".to_string(),
                occurred_at: "2026-07-18T10:00:00+00:00".to_string(),
            }),
        })
        .expect("complete message");
    assert_eq!(completed.message.status(), MessageStatus::Completed);
    assert!(fixture
        .store
        .events
        .lock()
        .expect("events")
        .iter()
        .any(|event| event == "complete-message:assistant-1:usage=true"));

    fixture.store.seed_message(message_record(
        "assistant-failed",
        session.id(),
        MessageRole::Assistant,
        MessageStatus::Streaming,
    ));
    let failed = fixture
        .service
        .fail_message(FailMessageRequest {
            message_id: "assistant-failed".to_string(),
            session_id: session.id().to_string(),
            error: "Codex CLI unavailable".to_string(),
        })
        .expect("fail message");
    assert_eq!(failed.message.status(), MessageStatus::Failed);
    assert_eq!(failed.error.as_deref(), Some("Codex CLI unavailable"));
    assert!(failed.content.is_empty());

    let exported = fixture
        .service
        .export_session(SessionExportRequest {
            session_id: session.id().to_string(),
            format: SessionExportFormat::Markdown,
            destination_directory: Some("D:\\exports".to_string()),
        })
        .expect("export session");
    assert_eq!(exported.status, "exported");
    let writes = fixture.files.writes.lock().expect("writes");
    assert_eq!(writes[0].1, "Fixture-Session-session-fixture.md");
    assert!(writes[0].2.contains("# Fixture Session"));
    assert!(writes[0].2.contains("### ASSISTANT - `completed`"));
}

#[test]
fn agent_runtime_updates_stream_state_through_session_owned_use_cases() {
    let fixture = fixture();
    let mut session = session_record(
        "session-runtime",
        "codex-cli",
        SessionLifecycle::Idle,
        false,
    );
    session.interaction_mode = "cli".to_string();
    fixture.store.seed_session(session);
    fixture.store.seed_message(message_record(
        "assistant-runtime",
        "session-runtime",
        MessageRole::Assistant,
        MessageStatus::Streaming,
    ));

    fixture
        .service
        .append_message_content("assistant-runtime", "first")
        .expect("append first token");
    fixture
        .service
        .append_message_content("assistant-runtime", " second")
        .expect("append second token");
    fixture
        .service
        .append_message_thinking("assistant-runtime", "reasoning")
        .expect("append thinking");
    fixture
        .service
        .append_message_tool_use(
            "assistant-runtime",
            serde_json::json!({"id": "tool-1", "name": "read", "status": "completed"}),
        )
        .expect("append tool use");
    fixture
        .service
        .append_message_rich_block(
            "assistant-runtime",
            serde_json::json!({"id": "block-1", "kind": "card", "v": 1, "text": "old"}),
        )
        .expect("append rich block");
    fixture
        .service
        .append_message_rich_block(
            "assistant-runtime",
            serde_json::json!({"id": "block-1", "kind": "card", "v": 1, "text": "new"}),
        )
        .expect("replace rich block");
    fixture
        .service
        .update_runtime_session_id("session-runtime", "provider-session-1")
        .expect("runtime session id");
    fixture
        .service
        .update_runtime_lifecycle("session-runtime", SessionLifecycle::Running)
        .expect("runtime lifecycle");

    let message = fixture
        .service
        .find_message("assistant-runtime")
        .expect("find message")
        .expect("message exists");
    assert_eq!(message.content, "first second");
    assert_eq!(message.thinking_content.as_deref(), Some("reasoning"));
    assert_eq!(message.tool_use.as_ref().map(Vec::len), Some(1));
    assert_eq!(message.rich_blocks.as_ref().map(Vec::len), Some(1));
    assert_eq!(
        message
            .rich_blocks
            .as_ref()
            .map(|blocks| &blocks[0]["text"]),
        Some(&serde_json::json!("new"))
    );
    let session = fixture
        .service
        .find_session("session-runtime")
        .expect("find session")
        .expect("session exists");
    assert_eq!(session.aggregate.lifecycle(), SessionLifecycle::Running);
    assert_eq!(
        session.runtime_session_id.as_deref(),
        Some("provider-session-1")
    );

    assert_eq!(
        fixture
            .service
            .cancel_streaming_messages("session-runtime")
            .expect("cancel streaming"),
        vec!["assistant-runtime".to_string()]
    );
    assert!(fixture
        .service
        .cancel_streaming_messages("session-runtime")
        .expect("deduplicate cancellation")
        .is_empty());
    assert_eq!(
        fixture
            .service
            .find_message("assistant-runtime")
            .expect("find cancelled message")
            .expect("cancelled message")
            .message
            .status(),
        MessageStatus::Cancelled
    );
    assert!(fixture
        .store
        .events
        .lock()
        .expect("events")
        .iter()
        .any(|event| event == "cancel-messages:1"));
}

#[test]
fn search_usage_and_maintenance_use_bounded_queries_and_deterministic_clock() {
    let fixture = fixture();
    let mut recoverable = session_record(
        "session-running",
        "codex-cli",
        SessionLifecycle::Running,
        false,
    );
    recoverable.updated_at = "2026-07-01T00:00:00+00:00".to_string();
    let inactive = session_record(
        "session-inactive",
        "codex-cli",
        SessionLifecycle::Idle,
        false,
    );
    let pinned = session_record("session-pinned", "codex-cli", SessionLifecycle::Idle, true);
    fixture.store.seed_session(recoverable);
    fixture.store.seed_session(inactive);
    fixture.store.seed_session(pinned);
    fixture
        .store
        .inactive_session_ids
        .lock()
        .expect("inactive ids")
        .extend(["session-inactive".to_string(), "session-pinned".to_string()]);

    assert!(fixture
        .service
        .search_sessions("   ", Some(999))
        .expect("empty search")
        .is_empty());
    let results = fixture
        .service
        .search_sessions(" fixture ", Some(999))
        .expect("search");
    assert_eq!(results.len(), 3);
    assert_eq!(
        fixture.store.search_queries.lock().expect("search queries")[0],
        SessionSearchQuery {
            text: "fixture".to_string(),
            limit: 100,
        }
    );
    fixture
        .service
        .list_messages(
            "session-inactive",
            Some(999),
            Some("message-cursor".to_string()),
        )
        .expect("bounded messages");
    fixture
        .service
        .list_messages("session-inactive", Some(-1), None)
        .expect("minimum message limit");
    assert_eq!(
        fixture
            .store
            .message_queries
            .lock()
            .expect("message queries")
            .as_slice(),
        [
            MessagePageQuery {
                session_id: "session-inactive".to_string(),
                limit: 200,
                before_id: Some("message-cursor".to_string()),
            },
            MessagePageQuery {
                session_id: "session-inactive".to_string(),
                limit: 1,
                before_id: None,
            },
        ]
    );

    let usage = fixture
        .service
        .usage_statistics(UsageStatisticsRange::Last7Days)
        .expect("usage");
    assert_eq!(usage.reported.total_tokens, 8);
    assert_eq!(
        fixture.store.usage_queries.lock().expect("usage queries")[0]
            .1
            .as_deref(),
        Some("2026-07-12T16:00:00+00:00")
    );

    let session_usage = fixture
        .service
        .session_usage_summary("session-inactive")
        .expect("session usage");
    assert_eq!(session_usage.session_id, "session-inactive");
    assert_eq!(session_usage.reported.total_tokens, 8);
    assert_eq!(
        fixture.store.usage_queries.lock().expect("usage queries")[1]
            .1
            .as_deref(),
        Some("session-inactive")
    );

    let before_unknown_query_count = fixture
        .store
        .usage_queries
        .lock()
        .expect("usage queries")
        .len();
    let error = fixture
        .service
        .session_usage_summary("session-missing")
        .expect_err("missing session");
    assert_eq!(
        error,
        SessionsApplicationError::SessionNotFound("session-missing".to_string())
    );
    assert_eq!(
        fixture
            .store
            .usage_queries
            .lock()
            .expect("usage queries")
            .len(),
        before_unknown_query_count
    );

    let result = fixture
        .service
        .run_maintenance(ArchivalPolicy {
            enabled: true,
            inactive_days: 10,
        })
        .expect("maintenance");
    assert_eq!(result.recovered, 1);
    assert_eq!(result.archived, 1);
    assert_eq!(
        SessionRepository::find(
            fixture.store.as_ref(),
            &SessionId::parse("session-running").expect("session id"),
        )
        .expect("find")
        .expect("running session")
        .aggregate
        .lifecycle(),
        SessionLifecycle::Failed
    );
    assert!(fixture
        .clock
        .calls
        .lock()
        .expect("clock calls")
        .contains(&"cutoff:10".to_string()));
    assert!(fixture
        .logging
        .entries
        .lock()
        .expect("log entries")
        .iter()
        .any(|entry| entry.category == "session.maintenance"));
}

#[test]
fn export_filenames_are_stable_and_safe_for_json_and_markdown() {
    let mut unsafe_title =
        session_record("session-export", "codex-cli", SessionLifecycle::Idle, false);
    unsafe_title.aggregate = SessionAggregate::rehydrate(
        SessionId::parse("session-export").expect("session id"),
        SessionTitle::for_creation(Some("  ../A:B*C?  ")),
        SessionLifecycle::Idle,
        SessionOwner::desktop(),
        None,
        false,
        false,
    );
    assert_eq!(
        super::service::safe_export_filename(&unsafe_title, SessionExportFormat::Json),
        "A-B-C-session-export.json"
    );

    unsafe_title.aggregate = SessionAggregate::rehydrate(
        SessionId::parse("session-export").expect("session id"),
        SessionTitle::for_creation(Some("会话")),
        SessionLifecycle::Idle,
        SessionOwner::desktop(),
        None,
        false,
        false,
    );
    assert_eq!(
        super::service::safe_export_filename(&unsafe_title, SessionExportFormat::Markdown),
        "session-session-export.md"
    );
}

#[test]
fn category_and_message_domain_failures_stop_before_persistence() {
    let fixture = fixture();
    fixture.store.seed_session(session_record(
        "session-fixture",
        "codex-cli",
        SessionLifecycle::Idle,
        false,
    ));
    fixture.store.categories.lock().expect("categories").insert(
        "category-existing".to_string(),
        CategoryRecord {
            category: SessionCategory::new(
                CategoryId::parse("category-existing").expect("category id"),
                CategoryName::parse("Work").expect("category name"),
                0,
            ),
            created_at: "100".to_string(),
            updated_at: "100".to_string(),
        },
    );

    assert_eq!(
        fixture.service.create_category("work".to_string()),
        Err(SessionsApplicationError::CategoryNameConflict(
            "work".to_string()
        ))
    );
    assert!(fixture
        .service
        .create_message(CreateMessageRequest {
            session_id: "session-fixture".to_string(),
            role: "user".to_string(),
            status: "completed".to_string(),
            content: "  ".to_string(),
            file_references: Vec::new(),
        })
        .is_err());
    assert!(fixture.store.messages.lock().expect("messages").is_empty());
}
