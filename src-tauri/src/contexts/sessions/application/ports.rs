use super::{
    CategoryRecord, ChatConfigurationValues, CreatedSessionWorktree, MessagePageQuery,
    MessageRecord, MessageUsageRecord, NewRemoteWorkspace, SessionApplicationLog,
    SessionChatConfiguration, SessionCreationOperation, SessionListScope, SessionProject,
    SessionRecord, SessionRemoteWorkspace, SessionSearchQuery, SessionSearchResult,
    SessionSshProfile, SessionUsageStatistics, SessionUsageSummary, SessionsApplicationError,
    UsageStatisticsRange,
};
use crate::contexts::sessions::domain::{
    CategoryId, ChatPreferences, MessageId, SessionActivation, SessionId,
};

pub(crate) trait SessionRepository: Send + Sync {
    fn find(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<SessionRecord>, SessionsApplicationError>;

    fn list(&self, scope: SessionListScope)
        -> Result<Vec<SessionRecord>, SessionsApplicationError>;

    #[cfg(test)]
    fn list_including_loop_owned(
        &self,
        scope: SessionListScope,
    ) -> Result<Vec<SessionRecord>, SessionsApplicationError> {
        self.list(scope)
    }

    fn search(
        &self,
        query: &SessionSearchQuery,
    ) -> Result<Vec<SessionSearchResult>, SessionsApplicationError>;

    fn active_session(&self) -> Result<Option<SessionRecord>, SessionsApplicationError>;

    fn save(&self, session: &SessionRecord) -> Result<SessionRecord, SessionsApplicationError>;

    fn recoverable_sessions(&self) -> Result<Vec<SessionRecord>, SessionsApplicationError>;

    fn inactive_sessions(
        &self,
        cutoff: &str,
    ) -> Result<Vec<SessionRecord>, SessionsApplicationError>;
}

pub(crate) trait SessionMessageRepository: Send + Sync {
    fn find(
        &self,
        message_id: &MessageId,
    ) -> Result<Option<MessageRecord>, SessionsApplicationError>;

    fn insert(&self, message: &MessageRecord) -> Result<MessageRecord, SessionsApplicationError>;

    fn save(&self, message: &MessageRecord) -> Result<MessageRecord, SessionsApplicationError>;

    fn save_stream_fields(&self, message: &MessageRecord) -> Result<(), SessionsApplicationError>;

    fn list(
        &self,
        query: &MessagePageQuery,
    ) -> Result<Vec<MessageRecord>, SessionsApplicationError>;

    fn list_all(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<MessageRecord>, SessionsApplicationError>;
}

pub(crate) trait SessionCategoryRepository: Send + Sync {
    fn list(&self) -> Result<Vec<CategoryRecord>, SessionsApplicationError>;

    fn find(
        &self,
        category_id: &CategoryId,
    ) -> Result<Option<CategoryRecord>, SessionsApplicationError>;

    fn name_exists(
        &self,
        name: &str,
        excluding: Option<&CategoryId>,
    ) -> Result<bool, SessionsApplicationError>;

    fn next_sort_order(&self) -> Result<i64, SessionsApplicationError>;

    fn insert(&self, category: &CategoryRecord)
        -> Result<CategoryRecord, SessionsApplicationError>;

    fn save(&self, category: &CategoryRecord) -> Result<CategoryRecord, SessionsApplicationError>;
}

pub(crate) trait SessionConfigurationRepository: Send + Sync {
    fn load(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<ChatConfigurationValues>, SessionsApplicationError>;

    fn save(
        &self,
        session_id: &SessionId,
        preferences: &ChatPreferences,
        updated_at: &str,
    ) -> Result<(), SessionsApplicationError>;
}

pub(crate) trait SessionUsageRepository: Send + Sync {
    fn statistics(
        &self,
        range: UsageStatisticsRange,
        range_start: Option<&str>,
        generated_at: &str,
    ) -> Result<SessionUsageStatistics, SessionsApplicationError>;

    fn summary_for_session(
        &self,
        session_id: &str,
        generated_at: &str,
    ) -> Result<SessionUsageSummary, SessionsApplicationError>;
}

pub(crate) trait SessionTransactionPort: Send + Sync {
    fn create_session(
        &self,
        session: &SessionRecord,
        activation: SessionActivation,
    ) -> Result<SessionRecord, SessionsApplicationError>;

    fn activate_session(
        &self,
        session: &SessionRecord,
    ) -> Result<SessionRecord, SessionsApplicationError>;

    fn archive_session(
        &self,
        session: &SessionRecord,
    ) -> Result<SessionRecord, SessionsApplicationError>;

    fn clear_active_session_if_matches(
        &self,
        session_id: &SessionId,
    ) -> Result<(), SessionsApplicationError>;

    fn delete_session(&self, session_id: &SessionId) -> Result<(), SessionsApplicationError>;

    fn delete_category(
        &self,
        category_id: &CategoryId,
        updated_at: &str,
    ) -> Result<(), SessionsApplicationError>;

    fn complete_message(
        &self,
        message: &MessageRecord,
        usage: Option<&MessageUsageRecord>,
    ) -> Result<MessageRecord, SessionsApplicationError>;

    fn save_runtime_session(
        &self,
        session: &SessionRecord,
    ) -> Result<SessionRecord, SessionsApplicationError>;

    fn cancel_messages(
        &self,
        messages: &[MessageRecord],
    ) -> Result<Vec<String>, SessionsApplicationError>;

    fn recover_orphaned_session(
        &self,
        session: &SessionRecord,
        recovered_at: &str,
    ) -> Result<(), SessionsApplicationError>;
}

pub(crate) trait SessionClockPort: Send + Sync {
    fn now(&self) -> String;

    fn inactivity_cutoff(&self, inactive_days: i64) -> Result<String, SessionsApplicationError>;

    fn usage_range_start(
        &self,
        range: UsageStatisticsRange,
    ) -> Result<Option<String>, SessionsApplicationError>;
}

pub(crate) trait SessionIdentityPort: Send + Sync {
    fn next_session_id(&self) -> String;
    fn next_message_id(&self) -> String;
    fn next_category_id(&self) -> String;
}

pub(crate) trait SessionCreationContextPort: Send + Sync {
    fn remote_workspace_uri(&self, workspace: &NewRemoteWorkspace) -> Option<String>;

    fn find_ssh_profile(
        &self,
        connection_id: &str,
    ) -> Result<Option<SessionSshProfile>, SessionsApplicationError>;

    fn ensure_agent_supports(
        &self,
        agent_id: &str,
        interaction_mode: &str,
    ) -> Result<(), SessionsApplicationError>;

    fn ensure_worktree_compatible(
        &self,
        remote_workspace_selected: bool,
        worktree_enabled: bool,
    ) -> Result<(), SessionsApplicationError>;

    fn prepare_project(&self, path: &str) -> Result<SessionProject, SessionsApplicationError>;

    fn normalize_remote_workspace(
        &self,
        workspace: &NewRemoteWorkspace,
    ) -> Result<SessionRemoteWorkspace, SessionsApplicationError>;

    fn remember_remote_workspace(
        &self,
        workspace: &SessionRemoteWorkspace,
    ) -> Result<(), SessionsApplicationError>;

    fn ensure_git_worktree_available(
        &self,
        project: &SessionProject,
    ) -> Result<(), SessionsApplicationError>;

    fn create_worktree(
        &self,
        project_path: &str,
        name: &str,
    ) -> Result<CreatedSessionWorktree, SessionsApplicationError>;
}

pub(crate) trait SessionRuntimePort: Send + Sync {
    fn stop_session_activity(&self, session_id: &str) -> Result<(), SessionsApplicationError>;
}

pub(crate) trait SessionFileContentPort: Send + Sync {
    fn read_reference_text(
        &self,
        session_id: &str,
        path: &str,
    ) -> Result<String, SessionsApplicationError>;

    fn write_export(
        &self,
        destination_directory: &str,
        filename: &str,
        content: &str,
    ) -> Result<String, SessionsApplicationError>;
}

pub(crate) trait SessionOperationPort: Send + Sync {
    fn start_session_creation(
        &self,
        related_entity_id: Option<String>,
    ) -> Result<SessionCreationOperation, SessionsApplicationError>;

    fn append_log(&self, operation_id: &str, line: String) -> Result<(), SessionsApplicationError>;

    fn complete_session_creation(
        &self,
        operation_id: &str,
        session: &SessionRecord,
    ) -> Result<(), SessionsApplicationError>;

    fn fail_session_creation(
        &self,
        operation_id: &str,
        error: String,
    ) -> Result<(), SessionsApplicationError>;
}

pub(crate) trait SessionLoggingPort: Send + Sync {
    fn write(&self, log: SessionApplicationLog) -> Result<(), SessionsApplicationError>;
}

pub(crate) trait SessionChatProfilePort: Send + Sync {
    fn defaults_for(
        &self,
        agent_id: &str,
    ) -> Result<ChatConfigurationValues, SessionsApplicationError>;
}

pub(crate) fn configuration_from_preferences(
    session: &SessionRecord,
    preferences: &ChatPreferences,
) -> SessionChatConfiguration {
    SessionChatConfiguration {
        session_id: session.id().to_string(),
        agent_id: session.agent_id.clone(),
        interaction_mode: session.interaction_mode.clone(),
        values: ChatConfigurationValues::from_preferences(preferences),
    }
}
