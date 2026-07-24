use crate::contexts::communications::domain::ConnectorKind;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum InteractionMode {
    Browser,
    NativeDesktop,
    Cli,
}

impl InteractionMode {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Browser => "browser",
            Self::NativeDesktop => "native-desktop",
            Self::Cli => "cli",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SessionLifecycleState {
    Idle,
    Starting,
    Running,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteWorkspace {
    pub(crate) host: String,
    pub(crate) port: Option<u16>,
    pub(crate) user: Option<String>,
    pub(crate) path: String,
    pub(crate) display_name: String,
    pub(crate) uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionSource {
    pub(crate) kind: String,
    pub(crate) connector: Option<ConnectorKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Session {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) agent_id: String,
    pub(crate) interaction_mode: InteractionMode,
    pub(crate) lifecycle_state: SessionLifecycleState,
    pub(crate) folder: Option<String>,
    pub(crate) project_path: Option<String>,
    pub(crate) worktree_path: Option<String>,
    pub(crate) worktree_name: Option<String>,
    pub(crate) worktree_branch: Option<String>,
    pub(crate) remote_workspace: Option<RemoteWorkspace>,
    pub(crate) remote_ssh_connection_id: Option<String>,
    pub(crate) remote_ssh_connection_revision: Option<i64>,
    pub(crate) runtime_session_id: Option<String>,
    pub(crate) category_id: Option<String>,
    pub(crate) source: SessionSource,
    pub(crate) pinned: bool,
    pub(crate) archived: bool,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateSessionInput {
    pub(crate) agent_id: String,
    pub(crate) interaction_mode: InteractionMode,
    pub(crate) title: Option<String>,
    pub(crate) folder: Option<String>,
    pub(crate) project_path: Option<String>,
    pub(crate) remote_workspace: Option<CreateRemoteWorkspaceInput>,
    pub(crate) worktree: Option<CreateWorktreeInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum ScheduledTaskFrequency {
    Minutes {
        interval: i64,
    },
    Hours {
        interval: i64,
    },
    Daily {
        time_of_day: String,
    },
    Weekly {
        weekday: i64,
        time_of_day: String,
    },
    Monthly {
        day_of_month: i64,
        time_of_day: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScheduledTask {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) content: String,
    pub(crate) agent_id: String,
    pub(crate) frequency: ScheduledTaskFrequency,
    pub(crate) enabled: bool,
    pub(crate) next_run_at: String,
    pub(crate) latest_status: String,
    pub(crate) latest_run_at: Option<String>,
    pub(crate) latest_run_session_id: Option<String>,
    pub(crate) latest_error: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateScheduledTaskInput {
    pub(crate) name: String,
    pub(crate) content: String,
    pub(crate) agent_id: String,
    pub(crate) frequency: ScheduledTaskFrequency,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetScheduledTaskEnabledInput {
    pub(crate) task_id: String,
    pub(crate) enabled: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateRemoteWorkspaceInput {
    pub(crate) host: String,
    pub(crate) port: Option<u16>,
    pub(crate) user: Option<String>,
    pub(crate) path: String,
    pub(crate) display_name: Option<String>,
    pub(crate) ssh_connection_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateWorktreeInput {
    pub(crate) enabled: bool,
    pub(crate) name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionSearchMatch {
    pub(crate) kind: String,
    pub(crate) excerpt: String,
    pub(crate) message_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionSearchResult {
    pub(crate) session: Session,
    pub(crate) matches: Vec<SessionSearchMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionCategory {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) sort_order: i64,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatConfig {
    pub(crate) agent_id: String,
    pub(crate) interaction_mode: InteractionMode,
    pub(crate) permission_mode: String,
    pub(crate) provider_id: Option<String>,
    pub(crate) model_id: Option<String>,
    pub(crate) reasoning_depth: Option<String>,
    pub(crate) streaming: bool,
    pub(crate) thinking: bool,
    pub(crate) long_context: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolUseBlock {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) input: Option<Value>,
    pub(crate) output: Option<Value>,
    pub(crate) status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TokenUsage {
    pub(crate) input: i64,
    pub(crate) output: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatFileReference {
    pub(crate) id: String,
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) size_bytes: Option<i64>,
    pub(crate) content_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatMessage {
    pub(crate) id: String,
    pub(crate) session_id: String,
    pub(crate) role: String,
    pub(crate) content: String,
    pub(crate) status: String,
    pub(crate) tool_use: Option<Vec<ToolUseBlock>>,
    pub(crate) thinking_content: Option<String>,
    pub(crate) rich_blocks: Option<Vec<Value>>,
    pub(crate) token_usage: Option<TokenUsage>,
    pub(crate) file_references: Option<Vec<ChatFileReference>>,
    pub(crate) error: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SessionExportFormat {
    Json,
    Markdown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionExportResult {
    pub(crate) status: String,
    pub(crate) path: Option<String>,
    pub(crate) content: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum UsageStatisticsRange {
    Today,
    Last7Days,
    Last30Days,
    All,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportedTokenTotals {
    pub(crate) input_tokens: i64,
    pub(crate) output_tokens: i64,
    pub(crate) cache_read_tokens: i64,
    pub(crate) cache_creation_tokens: i64,
    pub(crate) total_tokens: i64,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EstimatedCharacterTotals {
    pub(crate) input_characters: i64,
    pub(crate) output_characters: i64,
    pub(crate) total_characters: i64,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsageCoverage {
    pub(crate) reported_responses: i64,
    pub(crate) estimated_responses: i64,
    pub(crate) total_responses: i64,
    pub(crate) reported_percent: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsageStatisticsPoint {
    pub(crate) date: String,
    pub(crate) reported: ReportedTokenTotals,
    pub(crate) estimated: EstimatedCharacterTotals,
    pub(crate) response_count: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsageAgentBreakdown {
    pub(crate) agent_id: String,
    pub(crate) reported: ReportedTokenTotals,
    pub(crate) estimated: EstimatedCharacterTotals,
    pub(crate) response_count: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsageStatistics {
    pub(crate) range: UsageStatisticsRange,
    pub(crate) reported: ReportedTokenTotals,
    pub(crate) estimated: EstimatedCharacterTotals,
    pub(crate) coverage: UsageCoverage,
    pub(crate) counted_sessions: i64,
    pub(crate) daily: Vec<UsageStatisticsPoint>,
    pub(crate) by_agent: Vec<UsageAgentBreakdown>,
    pub(crate) generated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionUsageSummary {
    pub(crate) session_id: String,
    pub(crate) reported: ReportedTokenTotals,
    pub(crate) estimated: EstimatedCharacterTotals,
    pub(crate) coverage: UsageCoverage,
    pub(crate) response_count: i64,
    pub(crate) generated_at: String,
}
