use super::models::CreateSessionRequest;
use super::ports::configuration_from_preferences;
use super::{
    ArchivalPolicy, CategoryRecord, CompleteMessageRequest, CreateMessageRequest,
    FailMessageRequest, FileReferenceInput, MessagePageQuery, MessageRecord, MessageUsageRecord,
    NewSessionRequest, NewSessionWorkspace, PreparedNewSessionCreation, SessionApplicationLog,
    SessionApplicationLogLevel, SessionCategoryRepository, SessionChatConfiguration,
    SessionChatProfilePort, SessionClockPort, SessionConfigurationRepository,
    SessionCreationContextPort, SessionExportFormat, SessionExportRequest, SessionExportResult,
    SessionFileContentPort, SessionIdentityPort, SessionListScope, SessionLoggingPort,
    SessionMaintenanceResult, SessionMessageRepository, SessionOperationPort, SessionRecord,
    SessionRepository, SessionRuntimePort, SessionSearchQuery, SessionSearchResult,
    SessionTransactionPort, SessionUsageRepository, SessionUsageStatistics, SessionUsageSummary,
    SessionWorkspace, SessionsApplicationError, UsageStatisticsRange,
};
use crate::contexts::sessions::domain::{
    normalize_chat_preferences, restore_chat_preferences, CategoryId, CategoryName, FileReference,
    FileReferenceSet, MessageId, MessageRole, MessageStatus, SessionActivation, SessionAggregate,
    SessionCategory, SessionId, SessionLifecycle, SessionMessage, SessionTitle,
};
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct SessionApplicationPorts {
    pub(crate) sessions: Arc<dyn SessionRepository>,
    pub(crate) messages: Arc<dyn SessionMessageRepository>,
    pub(crate) categories: Arc<dyn SessionCategoryRepository>,
    pub(crate) configurations: Arc<dyn SessionConfigurationRepository>,
    pub(crate) usage: Arc<dyn SessionUsageRepository>,
    pub(crate) transactions: Arc<dyn SessionTransactionPort>,
    pub(crate) clock: Arc<dyn SessionClockPort>,
    pub(crate) identities: Arc<dyn SessionIdentityPort>,
    pub(crate) files: Arc<dyn SessionFileContentPort>,
    pub(crate) operations: Arc<dyn SessionOperationPort>,
    pub(crate) logging: Arc<dyn SessionLoggingPort>,
    pub(crate) chat_profiles: Arc<dyn SessionChatProfilePort>,
    pub(crate) creation: Arc<dyn SessionCreationContextPort>,
    pub(crate) runtime: Arc<dyn SessionRuntimePort>,
}

#[derive(Clone)]
pub(crate) struct SessionsApplicationService {
    ports: SessionApplicationPorts,
}

impl SessionsApplicationService {
    pub(crate) fn new(ports: SessionApplicationPorts) -> Self {
        Self { ports }
    }

    pub(crate) fn prepare_new_session_creation(
        &self,
        request: NewSessionRequest,
    ) -> Result<PreparedNewSessionCreation, SessionsApplicationError> {
        let related_entity_id = request
            .workspace
            .remote_workspace
            .as_ref()
            .and_then(|workspace| self.ports.creation.remote_workspace_uri(workspace))
            .or_else(|| request.workspace.project_path.clone())
            .or_else(|| request.workspace.folder.clone());
        let operation = self
            .ports
            .operations
            .start_session_creation(related_entity_id)?;
        Ok(PreparedNewSessionCreation { operation, request })
    }

    pub(crate) fn execute_new_session_creation(
        &self,
        prepared: PreparedNewSessionCreation,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        let operation_id = prepared.operation.id.clone();
        let result = self.create_new_session_record(prepared.request);
        self.finish_session_creation(&operation_id, result)
    }

    fn finish_session_creation(
        &self,
        operation_id: &str,
        result: Result<SessionRecord, SessionsApplicationError>,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        match result {
            Ok(session) => {
                let _ = self
                    .ports
                    .operations
                    .append_log(operation_id, format!("Created session {}", session.id()));
                let _ = self
                    .ports
                    .operations
                    .complete_session_creation(operation_id, &session);
                Ok(session)
            }
            Err(error) => {
                let message = error.to_string();
                let _ = self.ports.logging.write(SessionApplicationLog {
                    level: SessionApplicationLogLevel::Error,
                    category: "session.create".to_string(),
                    message: message.clone(),
                    session_id: None,
                    operation_id: Some(operation_id.to_string()),
                });
                let _ = self
                    .ports
                    .operations
                    .append_log(operation_id, message.clone());
                let _ = self
                    .ports
                    .operations
                    .fail_session_creation(operation_id, message);
                Err(error)
            }
        }
    }

    fn create_new_session_record(
        &self,
        request: NewSessionRequest,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        request.owner.validate_activation(request.activation)?;
        self.ports
            .creation
            .ensure_agent_supports(&request.agent_id, &request.interaction_mode)?;
        let workspace = self.prepare_new_session_workspace(&request.workspace)?;
        self.create_session_record(CreateSessionRequest {
            agent_id: request.agent_id,
            interaction_mode: request.interaction_mode,
            title: request.title,
            workspace,
            owner: request.owner,
            activation: request.activation,
        })
    }

    fn prepare_new_session_workspace(
        &self,
        request: &NewSessionWorkspace,
    ) -> Result<SessionWorkspace, SessionsApplicationError> {
        let remote_workspace = request
            .remote_workspace
            .as_ref()
            .map(|workspace| self.ports.creation.normalize_remote_workspace(workspace))
            .transpose()?;
        let worktree_enabled = request
            .worktree
            .as_ref()
            .is_some_and(|worktree| worktree.enabled);
        self.ports
            .creation
            .ensure_worktree_compatible(remote_workspace.is_some(), worktree_enabled)?;

        let selected_project = if remote_workspace.is_some() {
            None
        } else {
            request
                .project_path
                .as_deref()
                .or(request.folder.as_deref())
                .map(str::trim)
                .filter(|value| !value.is_empty())
        };
        let project = selected_project
            .map(|path| self.ports.creation.prepare_project(path))
            .transpose()?;
        if let Some(workspace) = &remote_workspace {
            self.ports.creation.remember_remote_workspace(workspace)?;
        }

        let mut workspace = SessionWorkspace {
            folder: project
                .as_ref()
                .map(|project| project.path.clone())
                .or_else(|| {
                    remote_workspace
                        .as_ref()
                        .map(|workspace| workspace.uri.clone())
                })
                .or_else(|| request.folder.clone()),
            project_path: project.as_ref().map(|project| project.path.clone()),
            remote_workspace,
            ..Default::default()
        };
        if worktree_enabled {
            let project = project.as_ref().ok_or_else(|| {
                SessionsApplicationError::Validation("Project unavailable".to_string())
            })?;
            self.ports.creation.ensure_git_worktree_available(project)?;
            let name = request
                .worktree
                .as_ref()
                .and_then(|worktree| worktree.name.as_deref())
                .unwrap_or("");
            let worktree = self.ports.creation.create_worktree(&project.path, name)?;
            workspace.folder = Some(worktree.path.clone());
            workspace.worktree_path = Some(worktree.path);
            workspace.worktree_name = Some(worktree.name);
            workspace.worktree_branch = Some(worktree.branch);
        }
        Ok(workspace)
    }

    fn create_session_record(
        &self,
        request: CreateSessionRequest,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        required_value(&request.agent_id, "Agent id")?;
        required_value(&request.interaction_mode, "Interaction mode")?;
        request.owner.validate_activation(request.activation)?;
        let id = SessionId::parse(self.ports.identities.next_session_id())?;
        let aggregate = SessionAggregate::create(
            id,
            SessionTitle::for_creation(request.title.as_deref()),
            request.owner,
        );
        let now = self.ports.clock.now();
        let record = SessionRecord {
            aggregate,
            agent_id: request.agent_id,
            interaction_mode: request.interaction_mode,
            workspace: request.workspace,
            runtime_session_id: None,
            created_at: now.clone(),
            updated_at: now,
        };
        self.ports
            .transactions
            .create_session(&record, request.activation)
    }

    pub(crate) fn list_sessions(
        &self,
        scope: SessionListScope,
    ) -> Result<Vec<SessionRecord>, SessionsApplicationError> {
        self.ports.sessions.list(scope)
    }

    pub(crate) fn search_sessions(
        &self,
        query: &str,
        limit: Option<i64>,
    ) -> Result<Vec<SessionSearchResult>, SessionsApplicationError> {
        let text = query.trim();
        if text.is_empty() {
            return Ok(Vec::new());
        }
        let query = SessionSearchQuery {
            text: text.to_string(),
            limit: limit.unwrap_or(50).clamp(1, 100) as usize,
        };
        self.ports.sessions.search(&query)
    }

    pub(crate) fn active_session(&self) -> Result<Option<SessionRecord>, SessionsApplicationError> {
        let Some(session) = self.ports.sessions.active_session()? else {
            return Ok(None);
        };
        if session.aggregate.is_archived() {
            self.ports
                .transactions
                .clear_active_session_if_matches(session.aggregate.id())?;
            return Ok(None);
        }
        Ok(Some(session))
    }

    pub(crate) fn switch_session(
        &self,
        session_id: &str,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        let session = self.load_session(session_id)?;
        session.aggregate.activation(SessionActivation::Activate)?;
        self.ports.transactions.activate_session(&session)
    }

    pub(crate) fn rename_session(
        &self,
        session_id: &str,
        title: String,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        let mut session = self.load_session(session_id)?;
        session.aggregate.rename(SessionTitle::for_rename(title)?);
        session.updated_at = self.ports.clock.now();
        self.ports.sessions.save(&session)
    }

    pub(crate) fn set_session_pinned(
        &self,
        session_id: &str,
        pinned: bool,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        let mut session = self.load_session(session_id)?;
        session.aggregate.set_pinned(pinned);
        session.updated_at = self.ports.clock.now();
        self.ports.sessions.save(&session)
    }

    pub(crate) fn set_session_archived(
        &self,
        session_id: &str,
        archived: bool,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        let mut session = self.load_session(session_id)?;
        if archived {
            self.ports.runtime.stop_session_activity(session_id)?;
            session.aggregate.archive();
        } else {
            session.aggregate.unarchive();
        }
        session.updated_at = self.ports.clock.now();
        if archived {
            self.ports.transactions.archive_session(&session)
        } else {
            self.ports.sessions.save(&session)
        }
    }

    pub(crate) fn delete_session(&self, session_id: &str) -> Result<(), SessionsApplicationError> {
        let session = self.load_session(session_id)?;
        self.ports.runtime.stop_session_activity(session_id)?;
        self.ports
            .transactions
            .delete_session(session.aggregate.id())
    }

    pub(crate) fn list_categories(&self) -> Result<Vec<CategoryRecord>, SessionsApplicationError> {
        self.ports.categories.list()
    }

    pub(crate) fn create_category(
        &self,
        name: String,
    ) -> Result<CategoryRecord, SessionsApplicationError> {
        let name = CategoryName::parse(name)?;
        if self.ports.categories.name_exists(name.as_str(), None)? {
            return Err(SessionsApplicationError::CategoryNameConflict(
                name.as_str().to_string(),
            ));
        }
        let id = CategoryId::parse(self.ports.identities.next_category_id())?;
        let category = SessionCategory::new(id, name, self.ports.categories.next_sort_order()?);
        let now = self.ports.clock.now();
        self.ports.categories.insert(&CategoryRecord {
            category,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub(crate) fn rename_category(
        &self,
        category_id: &str,
        name: String,
    ) -> Result<CategoryRecord, SessionsApplicationError> {
        let category_id = CategoryId::parse(category_id)?;
        let mut record = self.load_category(&category_id)?;
        let name = CategoryName::parse(name)?;
        if self
            .ports
            .categories
            .name_exists(name.as_str(), Some(&category_id))?
        {
            return Err(SessionsApplicationError::CategoryNameConflict(
                name.as_str().to_string(),
            ));
        }
        record.category.rename(name);
        record.updated_at = self.ports.clock.now();
        self.ports.categories.save(&record)
    }

    pub(crate) fn delete_category(
        &self,
        category_id: &str,
    ) -> Result<(), SessionsApplicationError> {
        let category_id = CategoryId::parse(category_id)?;
        self.load_category(&category_id)?;
        self.ports
            .transactions
            .delete_category(&category_id, &self.ports.clock.now())
    }

    pub(crate) fn assign_category(
        &self,
        session_id: &str,
        category_id: Option<&str>,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        let mut session = self.load_session(session_id)?;
        let category_id = category_id.map(CategoryId::parse).transpose()?;
        if let Some(category_id) = &category_id {
            self.load_category(category_id)?;
        }
        session.aggregate.assign_category(category_id);
        session.updated_at = self.ports.clock.now();
        self.ports.sessions.save(&session)
    }

    pub(crate) fn load_chat_configuration(
        &self,
        session_id: &str,
    ) -> Result<SessionChatConfiguration, SessionsApplicationError> {
        let session = self.load_session(session_id)?;
        let persisted = self.ports.configurations.load(session.aggregate.id())?;
        let preferences = persisted
            .as_ref()
            .and_then(|values| {
                restore_chat_preferences(&session.agent_id, values.as_domain_request())
            })
            .map(Ok)
            .unwrap_or_else(|| {
                let defaults = self.ports.chat_profiles.defaults_for(&session.agent_id)?;
                normalize_chat_preferences(&session.agent_id, defaults.as_domain_request())
                    .map_err(SessionsApplicationError::from)
            })?;
        Ok(configuration_from_preferences(&session, &preferences))
    }

    pub(crate) fn save_chat_configuration(
        &self,
        configuration: SessionChatConfiguration,
    ) -> Result<SessionChatConfiguration, SessionsApplicationError> {
        let session = self.load_session(&configuration.session_id)?;
        let preferences = normalize_chat_preferences(
            &session.agent_id,
            configuration.values.as_domain_request(),
        )?;
        self.ports.configurations.save(
            session.aggregate.id(),
            &preferences,
            &self.ports.clock.now(),
        )?;
        Ok(configuration_from_preferences(&session, &preferences))
    }

    pub(crate) fn validate_chat_configuration(
        &self,
        configuration: SessionChatConfiguration,
    ) -> Result<SessionChatConfiguration, SessionsApplicationError> {
        let session = self.load_session(&configuration.session_id)?;
        let preferences = normalize_chat_preferences(
            &session.agent_id,
            configuration.values.as_domain_request(),
        )?;
        Ok(configuration_from_preferences(&session, &preferences))
    }

    pub(crate) fn find_session(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionRecord>, SessionsApplicationError> {
        let session_id = SessionId::parse(session_id)?;
        self.ports.sessions.find(&session_id)
    }

    pub(crate) fn find_message(
        &self,
        message_id: &str,
    ) -> Result<Option<MessageRecord>, SessionsApplicationError> {
        let message_id = MessageId::parse(message_id)?;
        self.ports.messages.find(&message_id)
    }

    pub(crate) fn create_message(
        &self,
        request: CreateMessageRequest,
    ) -> Result<MessageRecord, SessionsApplicationError> {
        let session = self.load_session(&request.session_id)?;
        session.aggregate.ensure_accepts_messages()?;
        let role = MessageRole::parse(&request.role)?;
        let content = if role == MessageRole::User {
            let content = request.content.trim().to_string();
            if content.is_empty() {
                return Err(SessionsApplicationError::Validation(
                    "Message content cannot be empty.".to_string(),
                ));
            }
            content
        } else {
            request.content
        };
        let references = file_reference_set(request.file_references)?;
        let message = SessionMessage::rehydrate(
            MessageId::parse(self.ports.identities.next_message_id())?,
            session.aggregate.id().clone(),
            role,
            MessageStatus::parse(&request.status)?,
            references,
        );
        let now = self.ports.clock.now();
        self.ports.messages.insert(&MessageRecord {
            message,
            content,
            thinking_content: None,
            tool_use: None,
            rich_blocks: None,
            token_usage: None,
            error: None,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub(crate) fn compose_prompt(
        &self,
        session_id: &str,
        content: &str,
        references: Vec<FileReferenceInput>,
    ) -> Result<String, SessionsApplicationError> {
        self.load_session(session_id)?;
        let references = file_reference_set(references)?;
        if references.as_slice().is_empty() {
            return Ok(content.to_string());
        }
        let mut prompt = content.to_string();
        prompt.push_str("\n\nReferenced files:\n");
        for reference in references.as_slice() {
            let file_content = self
                .ports
                .files
                .read_reference_text(session_id, reference.path())?;
            prompt.push_str(&format!(
                "\n--- FILE: {} ---\n{}\n--- END FILE: {} ---\n",
                reference.path(),
                file_content,
                reference.path()
            ));
        }
        Ok(prompt)
    }

    pub(crate) fn list_messages(
        &self,
        session_id: &str,
        limit: Option<i64>,
        before_id: Option<String>,
    ) -> Result<Vec<MessageRecord>, SessionsApplicationError> {
        self.load_session(session_id)?;
        self.ports.messages.list(&MessagePageQuery {
            session_id: session_id.to_string(),
            limit: limit.unwrap_or(50).clamp(1, 200) as usize,
            before_id,
        })
    }

    pub(crate) fn complete_message(
        &self,
        request: CompleteMessageRequest,
    ) -> Result<MessageRecord, SessionsApplicationError> {
        let session_id = SessionId::parse(&request.session_id)?;
        let message_id = MessageId::parse(&request.message_id)?;
        let mut record = self.load_message(&message_id)?;
        record.message.ensure_owned_by(&session_id)?;
        record.message.transition_to(MessageStatus::Completed)?;
        validate_usage(request.usage.as_ref(), &message_id, &session_id)?;
        record.content = request.content;
        record.thinking_content = request.thinking_content;
        record.tool_use = request.tool_use;
        record.rich_blocks = request.rich_blocks;
        record.token_usage = request.token_usage;
        record.error = None;
        record.updated_at = self.ports.clock.now();
        self.ports
            .transactions
            .complete_message(&record, request.usage.as_ref())
    }

    pub(crate) fn fail_message(
        &self,
        request: FailMessageRequest,
    ) -> Result<MessageRecord, SessionsApplicationError> {
        let session_id = SessionId::parse(&request.session_id)?;
        let message_id = MessageId::parse(&request.message_id)?;
        let mut record = self.load_message(&message_id)?;
        record.message.ensure_owned_by(&session_id)?;
        record.message.transition_to(MessageStatus::Failed)?;
        record.error = Some(request.error);
        record.updated_at = self.ports.clock.now();
        self.ports.messages.save(&record)
    }

    pub(crate) fn append_message_content(
        &self,
        message_id: &str,
        content_delta: &str,
    ) -> Result<(), SessionsApplicationError> {
        let message_id = MessageId::parse(message_id)?;
        let mut record = self.load_message(&message_id)?;
        record.content.push_str(content_delta);
        record.updated_at = self.ports.clock.now();
        self.ports.messages.save_stream_fields(&record)
    }

    pub(crate) fn append_message_thinking(
        &self,
        message_id: &str,
        content_delta: &str,
    ) -> Result<(), SessionsApplicationError> {
        let message_id = MessageId::parse(message_id)?;
        let mut record = self.load_message(&message_id)?;
        record
            .thinking_content
            .get_or_insert_with(String::new)
            .push_str(content_delta);
        record.updated_at = self.ports.clock.now();
        self.ports.messages.save_stream_fields(&record)
    }

    pub(crate) fn append_message_tool_use(
        &self,
        message_id: &str,
        tool_use: Value,
    ) -> Result<(), SessionsApplicationError> {
        let message_id = MessageId::parse(message_id)?;
        let mut record = self.load_message(&message_id)?;
        record.tool_use.get_or_insert_with(Vec::new).push(tool_use);
        record.updated_at = self.ports.clock.now();
        self.ports.messages.save_stream_fields(&record)
    }

    pub(crate) fn append_message_rich_block(
        &self,
        message_id: &str,
        block: Value,
    ) -> Result<(), SessionsApplicationError> {
        let block_id = valid_rich_block_id(&block)?;
        let message_id = MessageId::parse(message_id)?;
        let mut record = self.load_message(&message_id)?;
        let blocks = record.rich_blocks.get_or_insert_with(Vec::new);
        if let Some(index) = blocks
            .iter()
            .position(|candidate| candidate.get("id").and_then(Value::as_str) == Some(block_id))
        {
            blocks[index] = block;
        } else {
            blocks.push(block);
        }
        record.updated_at = self.ports.clock.now();
        self.ports.messages.save_stream_fields(&record)
    }

    pub(crate) fn cancel_streaming_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<String>, SessionsApplicationError> {
        let session_id = SessionId::parse(session_id)?;
        self.ports
            .sessions
            .find(&session_id)?
            .ok_or_else(|| SessionsApplicationError::SessionNotFound(session_id.as_str().into()))?;
        let now = self.ports.clock.now();
        let mut cancelled = self
            .ports
            .messages
            .list_all(&session_id)?
            .into_iter()
            .filter(|record| record.message.status() == MessageStatus::Streaming)
            .collect::<Vec<_>>();
        for record in &mut cancelled {
            record.message.transition_to(MessageStatus::Cancelled)?;
            record.updated_at.clone_from(&now);
        }
        self.ports.transactions.cancel_messages(&cancelled)
    }

    pub(crate) fn update_runtime_lifecycle(
        &self,
        session_id: &str,
        lifecycle: SessionLifecycle,
    ) -> Result<(), SessionsApplicationError> {
        let mut session = self.load_session(session_id)?;
        session.aggregate.transition_to(lifecycle)?;
        session.updated_at = self.ports.clock.now();
        self.ports
            .transactions
            .save_runtime_session(&session)
            .map(|_| ())
    }

    pub(crate) fn update_runtime_session_id(
        &self,
        session_id: &str,
        runtime_session_id: &str,
    ) -> Result<(), SessionsApplicationError> {
        required_value(runtime_session_id, "Runtime session id")?;
        let mut session = self.load_session(session_id)?;
        session.runtime_session_id = Some(runtime_session_id.to_string());
        session.updated_at = self.ports.clock.now();
        self.ports
            .transactions
            .save_runtime_session(&session)
            .map(|_| ())
    }

    pub(crate) fn export_session(
        &self,
        request: SessionExportRequest,
    ) -> Result<SessionExportResult, SessionsApplicationError> {
        let Some(destination_directory) = request
            .destination_directory
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(SessionExportResult {
                status: "cancelled",
                path: None,
                content: None,
            });
        };
        let session = self.load_session(&request.session_id)?;
        let messages = self.ports.messages.list_all(session.aggregate.id())?;
        let payload = ExportPayload::from_records(&session, &messages, self.ports.clock.now());
        let content = payload.render(request.format)?;
        let filename = safe_export_filename(&session, request.format);
        let path = self
            .ports
            .files
            .write_export(destination_directory, &filename, &content)
            .inspect_err(|error| {
                let _ = self.ports.logging.write(SessionApplicationLog {
                    level: SessionApplicationLogLevel::Error,
                    category: "session.export".to_string(),
                    message: error.to_string(),
                    session_id: Some(session.id().to_string()),
                    operation_id: None,
                });
            })?;
        Ok(SessionExportResult {
            status: "exported",
            path: Some(path),
            content: None,
        })
    }

    pub(crate) fn usage_statistics(
        &self,
        range: UsageStatisticsRange,
    ) -> Result<SessionUsageStatistics, SessionsApplicationError> {
        let range_start = self.ports.clock.usage_range_start(range)?;
        self.ports
            .usage
            .statistics(range, range_start.as_deref(), &self.ports.clock.now())
    }

    pub(crate) fn session_usage_summary(
        &self,
        session_id: &str,
    ) -> Result<SessionUsageSummary, SessionsApplicationError> {
        let session = self.load_session(session_id)?;
        self.ports
            .usage
            .summary_for_session(session.id(), &self.ports.clock.now())
    }

    pub(crate) fn run_maintenance(
        &self,
        policy: ArchivalPolicy,
    ) -> Result<SessionMaintenanceResult, SessionsApplicationError> {
        let mut result = SessionMaintenanceResult::default();
        let recovered_at = self.ports.clock.now();
        for mut session in self.ports.sessions.recoverable_sessions()? {
            session.aggregate.transition_to(SessionLifecycle::Failed)?;
            session.updated_at.clone_from(&recovered_at);
            self.ports
                .transactions
                .recover_orphaned_session(&session, &recovered_at)?;
            let _ = self.ports.logging.write(SessionApplicationLog {
                level: SessionApplicationLogLevel::Warn,
                category: "session.runtime".to_string(),
                message: "Recovered orphan session state after startup.".to_string(),
                session_id: Some(session.id().to_string()),
                operation_id: None,
            });
            result.recovered += 1;
        }
        if policy.enabled {
            if policy.inactive_days <= 0 {
                return Err(SessionsApplicationError::Validation(
                    "Automatic archival inactivity days must be positive.".to_string(),
                ));
            }
            let cutoff = self.ports.clock.inactivity_cutoff(policy.inactive_days)?;
            let archived_at = self.ports.clock.now();
            for mut session in self.ports.sessions.inactive_sessions(&cutoff)? {
                if !session.aggregate.can_archive_automatically() {
                    continue;
                }
                session.aggregate.archive();
                session.updated_at.clone_from(&archived_at);
                self.ports.transactions.archive_session(&session)?;
                let _ = self.ports.logging.write(SessionApplicationLog {
                    level: SessionApplicationLogLevel::Info,
                    category: "session.runtime".to_string(),
                    message: "Automatically archived inactive session.".to_string(),
                    session_id: Some(session.id().to_string()),
                    operation_id: None,
                });
                result.archived += 1;
            }
        }
        if result.recovered > 0 || result.archived > 0 {
            let _ = self.ports.logging.write(SessionApplicationLog {
                level: SessionApplicationLogLevel::Info,
                category: "session.maintenance".to_string(),
                message: format!(
                    "Session maintenance completed. recovered={} archived={}",
                    result.recovered, result.archived
                ),
                session_id: None,
                operation_id: None,
            });
        }
        Ok(result)
    }

    fn load_session(&self, session_id: &str) -> Result<SessionRecord, SessionsApplicationError> {
        let session_id = SessionId::parse(session_id)?;
        self.ports
            .sessions
            .find(&session_id)?
            .ok_or_else(|| SessionsApplicationError::SessionNotFound(session_id.as_str().into()))
    }

    fn load_message(
        &self,
        message_id: &MessageId,
    ) -> Result<MessageRecord, SessionsApplicationError> {
        self.ports
            .messages
            .find(message_id)?
            .ok_or_else(|| SessionsApplicationError::MessageNotFound(message_id.as_str().into()))
    }

    fn load_category(
        &self,
        category_id: &CategoryId,
    ) -> Result<CategoryRecord, SessionsApplicationError> {
        self.ports
            .categories
            .find(category_id)?
            .ok_or_else(|| SessionsApplicationError::CategoryNotFound(category_id.as_str().into()))
    }
}

fn required_value(value: &str, name: &str) -> Result<(), SessionsApplicationError> {
    if value.trim().is_empty() {
        Err(SessionsApplicationError::Validation(format!(
            "{name} cannot be empty."
        )))
    } else {
        Ok(())
    }
}

fn valid_rich_block_id(block: &Value) -> Result<&str, SessionsApplicationError> {
    let Some(id) = block
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        return Err(SessionsApplicationError::Validation(
            "Invalid Rich Block payload.".to_string(),
        ));
    };
    let Some(_kind) = block
        .get("kind")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        return Err(SessionsApplicationError::Validation(
            "Invalid Rich Block payload.".to_string(),
        ));
    };
    if block.get("v").and_then(Value::as_i64) != Some(1) {
        return Err(SessionsApplicationError::Validation(
            "Invalid Rich Block payload.".to_string(),
        ));
    }
    Ok(id)
}

fn file_reference_set(
    references: Vec<FileReferenceInput>,
) -> Result<FileReferenceSet, SessionsApplicationError> {
    FileReferenceSet::new(
        references
            .into_iter()
            .map(|reference| {
                FileReference::new(
                    reference.id,
                    reference.path,
                    reference.name,
                    reference.size_bytes,
                    reference.content_hash,
                )
            })
            .collect::<Result<Vec<_>, _>>()?,
    )
    .map_err(Into::into)
}

fn validate_usage(
    usage: Option<&MessageUsageRecord>,
    message_id: &MessageId,
    session_id: &SessionId,
) -> Result<(), SessionsApplicationError> {
    let Some(usage) = usage else {
        return Ok(());
    };
    if usage.message_id != message_id.as_str() || usage.session_id != session_id.as_str() {
        return Err(SessionsApplicationError::Validation(
            "Usage records must be owned by the completed message and session.".to_string(),
        ));
    }
    if [
        usage.input_count,
        usage.output_count,
        usage.cache_read_count,
        usage.cache_creation_count,
    ]
    .into_iter()
    .any(|value| value < 0)
    {
        return Err(SessionsApplicationError::Validation(
            "Usage counts must be non-negative.".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn safe_export_filename(session: &SessionRecord, format: SessionExportFormat) -> String {
    let mut title = session
        .aggregate
        .title()
        .as_str()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    while title.contains("--") {
        title = title.replace("--", "-");
    }
    let title = title.trim_matches('-');
    let title = if title.is_empty() { "session" } else { title };
    format!("{}-{}.{}", title, session.id(), format.extension())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportPayload {
    version: i64,
    exported_at: String,
    session: ExportSession,
    messages: Vec<ExportMessage>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportSession {
    id: String,
    title: String,
    agent_id: String,
    interaction_mode: String,
    lifecycle_state: String,
    folder: Option<String>,
    project_path: Option<String>,
    worktree_path: Option<String>,
    worktree_name: Option<String>,
    worktree_branch: Option<String>,
    remote_workspace: Option<ExportRemoteWorkspace>,
    runtime_session_id: Option<String>,
    category_id: Option<String>,
    source: ExportSessionSource,
    pinned: bool,
    archived: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportRemoteWorkspace {
    host: String,
    user: Option<String>,
    path: String,
    display_name: String,
    uri: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportSessionSource {
    kind: String,
    connector: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportMessage {
    id: String,
    session_id: String,
    role: String,
    content: String,
    status: String,
    tool_use: Option<Vec<Value>>,
    thinking_content: Option<String>,
    rich_blocks: Option<Vec<Value>>,
    token_usage: Option<super::MessageTokenUsage>,
    file_references: Option<Vec<FileReferenceInput>>,
    error: Option<String>,
    created_at: String,
    updated_at: String,
}

impl ExportPayload {
    fn from_records(
        session: &SessionRecord,
        messages: &[MessageRecord],
        exported_at: String,
    ) -> Self {
        let workspace = &session.workspace;
        let remote_workspace =
            workspace
                .remote_workspace
                .as_ref()
                .map(|workspace| ExportRemoteWorkspace {
                    host: workspace.host.clone(),
                    user: workspace.user.clone(),
                    path: workspace.path.clone(),
                    display_name: workspace.display_name.clone(),
                    uri: workspace.uri.clone(),
                });
        let aggregate = &session.aggregate;
        Self {
            version: 1,
            exported_at,
            session: ExportSession {
                id: session.id().to_string(),
                title: aggregate.title().as_str().to_string(),
                agent_id: session.agent_id.clone(),
                interaction_mode: session.interaction_mode.clone(),
                lifecycle_state: aggregate.lifecycle().as_str().to_string(),
                folder: workspace.folder.clone(),
                project_path: workspace.project_path.clone(),
                worktree_path: workspace.worktree_path.clone(),
                worktree_name: workspace.worktree_name.clone(),
                worktree_branch: workspace.worktree_branch.clone(),
                remote_workspace,
                runtime_session_id: session.runtime_session_id.clone(),
                category_id: aggregate
                    .category_id()
                    .map(|category_id| category_id.as_str().to_string()),
                source: ExportSessionSource {
                    kind: aggregate.owner().kind().to_string(),
                    connector: aggregate
                        .owner()
                        .connector_id()
                        .and_then(export_connector)
                        .map(str::to_string),
                },
                pinned: aggregate.is_pinned(),
                archived: aggregate.is_archived(),
                created_at: session.created_at.clone(),
                updated_at: session.updated_at.clone(),
            },
            messages: messages.iter().map(ExportMessage::from_record).collect(),
        }
    }

    fn render(&self, format: SessionExportFormat) -> Result<String, SessionsApplicationError> {
        match format {
            SessionExportFormat::Json => serde_json::to_string_pretty(self)
                .map_err(|error| SessionsApplicationError::Serialization(error.to_string())),
            SessionExportFormat::Markdown => self.render_markdown(),
        }
    }

    fn render_markdown(&self) -> Result<String, SessionsApplicationError> {
        let mut output = String::new();
        output.push_str(&format!("# {}\n\n", self.session.title));
        output.push_str("## Session\n\n");
        output.push_str(&format!("- ID: `{}`\n", self.session.id));
        output.push_str(&format!("- Agent: `{}`\n", self.session.agent_id));
        output.push_str(&format!(
            "- Interaction mode: `{}`\n",
            self.session.interaction_mode
        ));
        output.push_str(&format!(
            "- Lifecycle: `{}`\n",
            self.session.lifecycle_state
        ));
        output.push_str(&format!("- Archived: `{}`\n", self.session.archived));
        output.push_str(&format!("- Pinned: `{}`\n", self.session.pinned));
        if let Some(category_id) = &self.session.category_id {
            output.push_str(&format!("- Category ID: `{category_id}`\n"));
        }
        if let Some(folder) = &self.session.folder {
            output.push_str(&format!("- Folder: `{folder}`\n"));
        }
        if let Some(project_path) = &self.session.project_path {
            output.push_str(&format!("- Project: `{project_path}`\n"));
        }
        output.push_str(&format!("- Created: `{}`\n", self.session.created_at));
        output.push_str(&format!("- Updated: `{}`\n", self.session.updated_at));
        output.push_str(&format!("- Exported: `{}`\n\n", self.exported_at));
        output.push_str("## Messages\n\n");
        for message in &self.messages {
            output.push_str(&format!(
                "### {} - `{}`\n\n",
                message.role.to_uppercase(),
                message.status
            ));
            output.push_str(&format!("- Message ID: `{}`\n", message.id));
            output.push_str(&format!("- Created: `{}`\n", message.created_at));
            if let Some(usage) = &message.token_usage {
                output.push_str(&format!(
                    "- Token usage: input `{}`, output `{}`\n",
                    usage.input, usage.output
                ));
            }
            if let Some(references) = &message.file_references {
                if !references.is_empty() {
                    output.push_str("- File references:\n");
                    for reference in references {
                        output.push_str(&format!("  - `{}`\n", reference.path));
                    }
                }
            }
            output.push('\n');
            output.push_str(&message.content);
            output.push_str("\n\n");
            if let Some(thinking) = &message.thinking_content {
                output.push_str("#### Thinking\n\n");
                output.push_str(&markdown_code_block("", thinking));
                output.push('\n');
            }
            if let Some(tool_use) = &message.tool_use {
                if !tool_use.is_empty() {
                    output.push_str("#### Tool Use\n\n");
                    let raw = serde_json::to_string_pretty(tool_use).map_err(|error| {
                        SessionsApplicationError::Serialization(error.to_string())
                    })?;
                    output.push_str(&markdown_code_block("json", &raw));
                    output.push('\n');
                }
            }
            if let Some(error) = &message.error {
                output.push_str("#### Error\n\n");
                output.push_str(&markdown_code_block("", error));
                output.push('\n');
            }
        }
        Ok(output)
    }
}

fn export_connector(connector: &str) -> Option<&'static str> {
    match connector {
        "feishu" => Some("feishu"),
        "telegram" => Some("telegram"),
        "dingtalk" => Some("ding-talk"),
        "wecom" => Some("we-com"),
        "weixin" | "wechat" => Some("weixin"),
        _ => None,
    }
}

impl ExportMessage {
    fn from_record(record: &MessageRecord) -> Self {
        let references = record.message.file_references();
        Self {
            id: record.message.id().as_str().to_string(),
            session_id: record.message.session_id().as_str().to_string(),
            role: record.message.role().as_str().to_string(),
            content: record.content.clone(),
            status: record.message.status().as_str().to_string(),
            tool_use: record.tool_use.clone(),
            thinking_content: record.thinking_content.clone(),
            rich_blocks: record.rich_blocks.clone(),
            token_usage: record.token_usage.clone(),
            file_references: (!references.as_slice().is_empty())
                .then(|| super::models::references_from_domain(references)),
            error: record.error.clone(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
        }
    }
}

fn markdown_code_block(language: &str, content: &str) -> String {
    let fence = if content.contains("```") {
        "````"
    } else {
        "```"
    };
    format!("{fence}{language}\n{content}\n{fence}\n")
}
