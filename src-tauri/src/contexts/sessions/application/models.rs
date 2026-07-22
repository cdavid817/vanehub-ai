use crate::contexts::sessions::domain::{
    ChatConfigurationRequest, ChatPreferences, FileReferenceSet, SessionActivation,
    SessionAggregate, SessionCategory, SessionMessage, SessionOwner,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SessionWorkspace {
    pub(crate) folder: Option<String>,
    pub(crate) project_path: Option<String>,
    pub(crate) worktree_path: Option<String>,
    pub(crate) worktree_name: Option<String>,
    pub(crate) worktree_branch: Option<String>,
    pub(crate) remote_workspace: Option<SessionRemoteWorkspace>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionRemoteWorkspace {
    pub(crate) host: String,
    pub(crate) port: Option<u16>,
    pub(crate) user: Option<String>,
    pub(crate) path: String,
    pub(crate) display_name: String,
    pub(crate) uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionRecord {
    pub(crate) aggregate: SessionAggregate,
    pub(crate) agent_id: String,
    pub(crate) interaction_mode: String,
    pub(crate) workspace: SessionWorkspace,
    pub(crate) runtime_session_id: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl SessionRecord {
    pub(crate) fn id(&self) -> &str {
        self.aggregate.id().as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeSessionSnapshot {
    pub(crate) id: String,
    pub(crate) agent_id: String,
    pub(crate) interaction_mode: String,
    pub(crate) lifecycle: String,
    pub(crate) folder: Option<String>,
    pub(crate) runtime_session_id: Option<String>,
    pub(crate) archived: bool,
}

impl RuntimeSessionSnapshot {
    pub(crate) fn from_record(record: &SessionRecord) -> Self {
        Self {
            id: record.id().to_string(),
            agent_id: record.agent_id.clone(),
            interaction_mode: record.interaction_mode.clone(),
            lifecycle: record.aggregate.lifecycle().as_str().to_string(),
            folder: record.workspace.folder.clone(),
            runtime_session_id: record.runtime_session_id.clone(),
            archived: record.aggregate.is_archived(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CreateSessionRequest {
    pub(crate) agent_id: String,
    pub(crate) interaction_mode: String,
    pub(crate) title: Option<String>,
    pub(crate) workspace: SessionWorkspace,
    pub(crate) owner: SessionOwner,
    pub(crate) activation: SessionActivation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewSessionRequest {
    pub(crate) agent_id: String,
    pub(crate) interaction_mode: String,
    pub(crate) title: Option<String>,
    pub(crate) workspace: NewSessionWorkspace,
    pub(crate) owner: SessionOwner,
    pub(crate) activation: SessionActivation,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct NewSessionWorkspace {
    pub(crate) folder: Option<String>,
    pub(crate) project_path: Option<String>,
    pub(crate) remote_workspace: Option<NewRemoteWorkspace>,
    pub(crate) worktree: Option<NewWorktree>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewRemoteWorkspace {
    pub(crate) host: String,
    pub(crate) port: Option<u16>,
    pub(crate) user: Option<String>,
    pub(crate) path: String,
    pub(crate) display_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewWorktree {
    pub(crate) enabled: bool,
    pub(crate) name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionProject {
    pub(crate) path: String,
    pub(crate) is_git: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreatedSessionWorktree {
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) branch: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionListScope {
    Current,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionSearchQuery {
    pub(crate) text: String,
    pub(crate) limit: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionSearchMatchKind {
    Title,
    Project,
    Message,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionSearchMatch {
    pub(crate) kind: SessionSearchMatchKind,
    pub(crate) excerpt: String,
    pub(crate) message_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionSearchResult {
    pub(crate) session: SessionRecord,
    pub(crate) matches: Vec<SessionSearchMatch>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CategoryRecord {
    pub(crate) category: SessionCategory,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatConfigurationValues {
    pub(crate) permission_mode: String,
    pub(crate) provider_id: Option<String>,
    pub(crate) model_id: Option<String>,
    pub(crate) reasoning_depth: Option<String>,
    pub(crate) streaming: bool,
    pub(crate) thinking: bool,
    pub(crate) long_context: bool,
}

impl ChatConfigurationValues {
    pub(super) fn as_domain_request(&self) -> ChatConfigurationRequest<'_> {
        ChatConfigurationRequest {
            permission_mode: &self.permission_mode,
            provider_id: self.provider_id.as_deref(),
            model_id: self.model_id.as_deref(),
            reasoning_depth: self.reasoning_depth.as_deref(),
            streaming: self.streaming,
            thinking: self.thinking,
            long_context: self.long_context,
        }
    }

    pub(crate) fn from_preferences(preferences: &ChatPreferences) -> Self {
        Self {
            permission_mode: preferences.permission_mode().to_string(),
            provider_id: Some(preferences.provider_id().to_string()),
            model_id: Some(preferences.model_id().to_string()),
            reasoning_depth: preferences.reasoning_depth().map(str::to_string),
            streaming: preferences.streaming(),
            thinking: preferences.thinking(),
            long_context: preferences.long_context(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionChatConfiguration {
    pub(crate) session_id: String,
    pub(crate) agent_id: String,
    pub(crate) interaction_mode: String,
    pub(crate) values: ChatConfigurationValues,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileReferenceInput {
    pub(crate) id: String,
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) size_bytes: Option<i64>,
    pub(crate) content_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CreateMessageRequest {
    pub(crate) session_id: String,
    pub(crate) role: String,
    pub(crate) status: String,
    pub(crate) content: String,
    pub(crate) file_references: Vec<FileReferenceInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MessageTokenUsage {
    pub(crate) input: i64,
    pub(crate) output: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MessageRecord {
    pub(crate) message: SessionMessage,
    pub(crate) content: String,
    pub(crate) thinking_content: Option<String>,
    pub(crate) tool_use: Option<Vec<Value>>,
    pub(crate) rich_blocks: Option<Vec<Value>>,
    pub(crate) token_usage: Option<MessageTokenUsage>,
    pub(crate) error: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeFileReferenceSnapshot {
    pub(crate) id: String,
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) size_bytes: Option<i64>,
    pub(crate) content_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RuntimeMessageSnapshot {
    pub(crate) id: String,
    pub(crate) session_id: String,
    pub(crate) role: String,
    pub(crate) status: String,
    pub(crate) content: String,
    pub(crate) thinking_content: Option<String>,
    pub(crate) tool_use: Vec<Value>,
    pub(crate) rich_blocks: Vec<Value>,
    pub(crate) token_usage: Option<MessageTokenUsage>,
    pub(crate) file_references: Vec<RuntimeFileReferenceSnapshot>,
    pub(crate) error: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl RuntimeMessageSnapshot {
    pub(crate) fn from_record(record: &MessageRecord) -> Self {
        Self {
            id: record.message.id().as_str().to_string(),
            session_id: record.message.session_id().as_str().to_string(),
            role: record.message.role().as_str().to_string(),
            status: record.message.status().as_str().to_string(),
            content: record.content.clone(),
            thinking_content: record.thinking_content.clone(),
            tool_use: record.tool_use.clone().unwrap_or_default(),
            rich_blocks: record.rich_blocks.clone().unwrap_or_default(),
            token_usage: record.token_usage.clone(),
            file_references: record
                .message
                .file_references()
                .as_slice()
                .iter()
                .map(|reference| RuntimeFileReferenceSnapshot {
                    id: reference.id().to_string(),
                    path: reference.path().to_string(),
                    name: reference.name().to_string(),
                    size_bytes: reference.size_bytes(),
                    content_hash: reference.content_hash().map(str::to_string),
                })
                .collect(),
            error: record.error.clone(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompleteMessageRequest {
    pub(crate) message_id: String,
    pub(crate) session_id: String,
    pub(crate) content: String,
    pub(crate) thinking_content: Option<String>,
    pub(crate) tool_use: Option<Vec<Value>>,
    pub(crate) rich_blocks: Option<Vec<Value>>,
    pub(crate) token_usage: Option<MessageTokenUsage>,
    pub(crate) usage: Option<MessageUsageRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FailMessageRequest {
    pub(crate) message_id: String,
    pub(crate) session_id: String,
    pub(crate) error: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MessagePageQuery {
    pub(crate) session_id: String,
    pub(crate) limit: usize,
    pub(crate) before_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SessionExportFormat {
    Json,
    Markdown,
}

impl SessionExportFormat {
    pub(super) fn extension(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Markdown => "md",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionExportRequest {
    pub(crate) session_id: String,
    pub(crate) format: SessionExportFormat,
    pub(crate) destination_directory: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionExportResult {
    pub(crate) status: &'static str,
    pub(crate) path: Option<String>,
    pub(crate) content: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionCreationOperation {
    pub(crate) id: String,
    pub(crate) related_entity_id: Option<String>,
    pub(crate) message: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedNewSessionCreation {
    pub(crate) operation: SessionCreationOperation,
    pub(super) request: NewSessionRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum UsageStatisticsRange {
    Today,
    Last7Days,
    Last30Days,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionUsageAccountingKind {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "persisted reported-token records remain supported"
        )
    )]
    Reported,
    Estimated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionUsageUnit {
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "persisted token-unit records remain supported")
    )]
    Tokens,
    Characters,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MessageUsageRecord {
    pub(crate) message_id: String,
    pub(crate) session_id: String,
    pub(crate) agent_id: String,
    pub(crate) provider_id: Option<String>,
    pub(crate) model_id: Option<String>,
    pub(crate) accounting_kind: SessionUsageAccountingKind,
    pub(crate) unit: SessionUsageUnit,
    pub(crate) input_count: i64,
    pub(crate) output_count: i64,
    pub(crate) cache_read_count: i64,
    pub(crate) cache_creation_count: i64,
    pub(crate) source: String,
    pub(crate) occurred_at: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportedTokenTotals {
    pub(crate) input_tokens: i64,
    pub(crate) output_tokens: i64,
    pub(crate) cache_read_tokens: i64,
    pub(crate) cache_creation_tokens: i64,
    pub(crate) total_tokens: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EstimatedCharacterTotals {
    pub(crate) input_characters: i64,
    pub(crate) output_characters: i64,
    pub(crate) total_characters: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionUsageCoverage {
    pub(crate) reported_responses: i64,
    pub(crate) estimated_responses: i64,
    pub(crate) total_responses: i64,
    pub(crate) reported_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionUsagePoint {
    pub(crate) date: String,
    pub(crate) reported: ReportedTokenTotals,
    pub(crate) estimated: EstimatedCharacterTotals,
    pub(crate) response_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionUsageAgentBreakdown {
    pub(crate) agent_id: String,
    pub(crate) reported: ReportedTokenTotals,
    pub(crate) estimated: EstimatedCharacterTotals,
    pub(crate) response_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionUsageStatistics {
    pub(crate) range: UsageStatisticsRange,
    pub(crate) reported: ReportedTokenTotals,
    pub(crate) estimated: EstimatedCharacterTotals,
    pub(crate) coverage: SessionUsageCoverage,
    pub(crate) counted_sessions: i64,
    pub(crate) daily: Vec<SessionUsagePoint>,
    pub(crate) by_agent: Vec<SessionUsageAgentBreakdown>,
    pub(crate) generated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionUsageSummary {
    pub(crate) session_id: String,
    pub(crate) reported: ReportedTokenTotals,
    pub(crate) estimated: EstimatedCharacterTotals,
    pub(crate) coverage: SessionUsageCoverage,
    pub(crate) response_count: i64,
    pub(crate) generated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ArchivalPolicy {
    pub(crate) enabled: bool,
    pub(crate) inactive_days: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SessionMaintenanceResult {
    pub(crate) recovered: usize,
    pub(crate) archived: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionApplicationLogLevel {
    Error,
    Warn,
    Info,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "session logging preserves the four-level log contract"
        )
    )]
    Debug,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionApplicationLog {
    pub(crate) level: SessionApplicationLogLevel,
    pub(crate) category: String,
    pub(crate) message: String,
    pub(crate) session_id: Option<String>,
    pub(crate) operation_id: Option<String>,
}

pub(super) fn references_from_domain(references: &FileReferenceSet) -> Vec<FileReferenceInput> {
    references
        .as_slice()
        .iter()
        .map(|reference| FileReferenceInput {
            id: reference.id().to_string(),
            path: reference.path().to_string(),
            name: reference.name().to_string(),
            size_bytes: reference.size_bytes(),
            content_hash: reference.content_hash().map(str::to_string),
        })
        .collect()
}
