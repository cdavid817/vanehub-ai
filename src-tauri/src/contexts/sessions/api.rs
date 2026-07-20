use super::application::SessionsApplicationService;
pub(crate) use super::application::{
    ArchivalPolicy, CategoryRecord, ChatConfigurationValues, CompleteMessageRequest,
    CreateMessageRequest, FailMessageRequest, FileReferenceInput, MessageRecord, MessageTokenUsage,
    MessageUsageRecord, NewRemoteWorkspace, NewSessionRequest, NewSessionWorkspace, NewWorktree,
    PreparedNewSessionCreation, RuntimeMessageSnapshot, RuntimeSessionSnapshot,
    SessionChatConfiguration, SessionCreationOperation, SessionExportFormat, SessionExportRequest,
    SessionExportResult, SessionListScope, SessionMaintenanceResult, SessionRecord,
    SessionSearchMatchKind, SessionSearchResult, SessionUsageAccountingKind,
    SessionUsageStatistics, SessionUsageSummary, SessionUsageUnit,
    SessionsApplicationError as SessionsError, UsageStatisticsRange,
};
pub(crate) use super::domain::{SessionActivation, SessionLifecycle, SessionOwner};
use serde_json::Value;

#[derive(Clone)]
pub(crate) struct SessionsApi {
    service: SessionsApplicationService,
}

impl SessionsApi {
    pub(crate) fn new(service: SessionsApplicationService) -> Self {
        Self { service }
    }

    pub(crate) fn prepare_creation(
        &self,
        request: NewSessionRequest,
    ) -> Result<PreparedNewSessionCreation, SessionsError> {
        self.service.prepare_new_session_creation(request)
    }

    pub(crate) fn execute_creation(
        &self,
        prepared: PreparedNewSessionCreation,
    ) -> Result<SessionRecord, SessionsError> {
        self.service.execute_new_session_creation(prepared)
    }

    pub(crate) fn list_current(&self) -> Result<Vec<SessionRecord>, SessionsError> {
        self.service.list_sessions(SessionListScope::Current)
    }

    pub(crate) fn list_archived(&self) -> Result<Vec<SessionRecord>, SessionsError> {
        self.service.list_sessions(SessionListScope::Archived)
    }

    pub(crate) fn search(
        &self,
        query: &str,
        limit: Option<i64>,
    ) -> Result<Vec<SessionSearchResult>, SessionsError> {
        self.service.search_sessions(query, limit)
    }

    pub(crate) fn active(&self) -> Result<Option<SessionRecord>, SessionsError> {
        self.service.active_session()
    }

    pub(crate) fn switch(&self, session_id: &str) -> Result<SessionRecord, SessionsError> {
        self.service.switch_session(session_id)
    }

    pub(crate) fn rename(
        &self,
        session_id: &str,
        title: String,
    ) -> Result<SessionRecord, SessionsError> {
        self.service.rename_session(session_id, title)
    }

    pub(crate) fn set_pinned(
        &self,
        session_id: &str,
        pinned: bool,
    ) -> Result<SessionRecord, SessionsError> {
        self.service.set_session_pinned(session_id, pinned)
    }

    pub(crate) fn set_archived(
        &self,
        session_id: &str,
        archived: bool,
    ) -> Result<SessionRecord, SessionsError> {
        self.service.set_session_archived(session_id, archived)
    }

    pub(crate) fn delete(&self, session_id: &str) -> Result<(), SessionsError> {
        self.service.delete_session(session_id)
    }

    pub(crate) fn list_categories(&self) -> Result<Vec<CategoryRecord>, SessionsError> {
        self.service.list_categories()
    }

    pub(crate) fn create_category(&self, name: String) -> Result<CategoryRecord, SessionsError> {
        self.service.create_category(name)
    }

    pub(crate) fn rename_category(
        &self,
        category_id: &str,
        name: String,
    ) -> Result<CategoryRecord, SessionsError> {
        self.service.rename_category(category_id, name)
    }

    pub(crate) fn delete_category(&self, category_id: &str) -> Result<(), SessionsError> {
        self.service.delete_category(category_id)
    }

    pub(crate) fn assign_category(
        &self,
        session_id: &str,
        category_id: Option<&str>,
    ) -> Result<SessionRecord, SessionsError> {
        self.service.assign_category(session_id, category_id)
    }

    pub(crate) fn load_chat_configuration(
        &self,
        session_id: &str,
    ) -> Result<SessionChatConfiguration, SessionsError> {
        self.service.load_chat_configuration(session_id)
    }

    pub(crate) fn save_chat_configuration(
        &self,
        configuration: SessionChatConfiguration,
    ) -> Result<SessionChatConfiguration, SessionsError> {
        self.service.save_chat_configuration(configuration)
    }

    pub(crate) fn validate_chat_configuration(
        &self,
        configuration: SessionChatConfiguration,
    ) -> Result<SessionChatConfiguration, SessionsError> {
        self.service.validate_chat_configuration(configuration)
    }

    pub(crate) fn runtime_session(
        &self,
        session_id: &str,
    ) -> Result<Option<RuntimeSessionSnapshot>, SessionsError> {
        self.service
            .find_session(session_id)
            .map(|record| record.as_ref().map(RuntimeSessionSnapshot::from_record))
    }

    pub(crate) fn runtime_message(
        &self,
        message_id: &str,
    ) -> Result<Option<RuntimeMessageSnapshot>, SessionsError> {
        self.service
            .find_message(message_id)
            .map(|record| record.as_ref().map(RuntimeMessageSnapshot::from_record))
    }

    pub(crate) fn create_message(
        &self,
        request: CreateMessageRequest,
    ) -> Result<MessageRecord, SessionsError> {
        self.service.create_message(request)
    }

    pub(crate) fn compose_prompt(
        &self,
        session_id: &str,
        content: &str,
        references: Vec<FileReferenceInput>,
    ) -> Result<String, SessionsError> {
        self.service.compose_prompt(session_id, content, references)
    }

    pub(crate) fn list_messages(
        &self,
        session_id: &str,
        limit: Option<i64>,
        before_id: Option<String>,
    ) -> Result<Vec<MessageRecord>, SessionsError> {
        self.service.list_messages(session_id, limit, before_id)
    }

    pub(crate) fn complete_message(
        &self,
        request: CompleteMessageRequest,
    ) -> Result<MessageRecord, SessionsError> {
        self.service.complete_message(request)
    }

    pub(crate) fn fail_message(
        &self,
        request: FailMessageRequest,
    ) -> Result<MessageRecord, SessionsError> {
        self.service.fail_message(request)
    }

    pub(crate) fn append_message_content(
        &self,
        message_id: &str,
        content_delta: &str,
    ) -> Result<(), SessionsError> {
        self.service
            .append_message_content(message_id, content_delta)
    }

    pub(crate) fn append_message_thinking(
        &self,
        message_id: &str,
        content_delta: &str,
    ) -> Result<(), SessionsError> {
        self.service
            .append_message_thinking(message_id, content_delta)
    }

    pub(crate) fn append_message_tool_use(
        &self,
        message_id: &str,
        tool_use: Value,
    ) -> Result<(), SessionsError> {
        self.service.append_message_tool_use(message_id, tool_use)
    }

    pub(crate) fn append_message_rich_block(
        &self,
        message_id: &str,
        block: Value,
    ) -> Result<(), SessionsError> {
        self.service.append_message_rich_block(message_id, block)
    }

    pub(crate) fn cancel_streaming_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<String>, SessionsError> {
        self.service.cancel_streaming_messages(session_id)
    }

    pub(crate) fn update_runtime_lifecycle(
        &self,
        session_id: &str,
        lifecycle: SessionLifecycle,
    ) -> Result<(), SessionsError> {
        self.service.update_runtime_lifecycle(session_id, lifecycle)
    }

    pub(crate) fn update_runtime_session_id(
        &self,
        session_id: &str,
        runtime_session_id: &str,
    ) -> Result<(), SessionsError> {
        self.service
            .update_runtime_session_id(session_id, runtime_session_id)
    }

    pub(crate) fn export(
        &self,
        request: SessionExportRequest,
    ) -> Result<SessionExportResult, SessionsError> {
        self.service.export_session(request)
    }

    pub(crate) fn usage_statistics(
        &self,
        range: UsageStatisticsRange,
    ) -> Result<SessionUsageStatistics, SessionsError> {
        self.service.usage_statistics(range)
    }

    pub(crate) fn session_usage_summary(
        &self,
        session_id: &str,
    ) -> Result<SessionUsageSummary, SessionsError> {
        self.service.session_usage_summary(session_id)
    }

    pub(crate) fn run_maintenance(
        &self,
        policy: ArchivalPolicy,
    ) -> Result<SessionMaintenanceResult, SessionsError> {
        self.service.run_maintenance(policy)
    }
}
