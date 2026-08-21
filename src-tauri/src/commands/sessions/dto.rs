use crate::contexts::communications::domain::ConnectorKind;
use crate::contexts::sessions::api::{
    RecoveryDecision, RecoveryEvidenceReference, RecoveryReasonCode, RecoveryTrigger,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum InteractionMode {
    Browser,
    NativeDesktop,
    Cli,
    Api,
}

impl InteractionMode {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Browser => "browser",
            Self::NativeDesktop => "native-desktop",
            Self::Cli => "cli",
            Self::Api => "api",
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
pub(crate) struct SessionExecutionOrigin {
    pub(crate) kind: String,
    pub(crate) id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Session {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) agent_id: String,
    pub(crate) seats: Vec<SessionSeat>,
    pub(crate) interaction_mode: InteractionMode,
    pub(crate) lifecycle_state: SessionLifecycleState,
    pub(crate) recovery_status: SessionRecoveryStatus,
    pub(crate) recovery_revision: u64,
    pub(crate) state_revision: u64,
    pub(crate) history_revision: u64,
    pub(crate) active_execution_run_id: Option<String>,
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
    pub(crate) execution_origin: SessionExecutionOrigin,
    pub(crate) pinned: bool,
    pub(crate) archived: bool,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SessionRecoveryStatus {
    Clean,
    Reconciling,
    ActionRequired,
    Quarantined,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionRecoveryReport {
    pub(crate) report_id: String,
    pub(crate) session_id: String,
    pub(crate) recovery_revision: u64,
    pub(crate) trigger: RecoveryTrigger,
    pub(crate) observed_lifecycle: String,
    pub(crate) observed_execution_run_id: Option<String>,
    pub(crate) decision: RecoveryDecision,
    pub(crate) reason_codes: Vec<RecoveryReasonCode>,
    pub(crate) evidence_refs: Vec<RecoveryEvidenceReference>,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionRecoverySummary {
    pub(crate) session: Session,
    pub(crate) latest_report: Option<SessionRecoveryReport>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionRecoveryAcknowledgement {
    pub(crate) session: Session,
    pub(crate) report: SessionRecoveryReport,
}

/// One participant in a session: an Agent playing an expert role.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionSeat {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) seat_id: Option<String>,
    pub(crate) agent_id: String,
    pub(crate) role_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) role_snapshot: Option<SessionSeatRoleSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) joined_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) left_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionSeatRoleSnapshot {
    pub(crate) role_name: Option<String>,
    pub(crate) avatar: String,
    pub(crate) color: String,
    pub(crate) responsibility: Option<String>,
    pub(crate) agent_name: String,
    pub(crate) model_family: String,
    pub(crate) cross_family_reviewer: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateSessionSeatsInput {
    pub(crate) session_id: String,
    pub(crate) expected_updated_at: String,
    pub(crate) seats: Vec<SessionSeat>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateSessionInput {
    pub(crate) agent_id: String,
    /// Omitted for a single-Agent session, which the native layer records as one seat built from
    /// `agent_id`.
    #[serde(default)]
    pub(crate) seats: Vec<SessionSeat>,
    pub(crate) interaction_mode: InteractionMode,
    pub(crate) title: Option<String>,
    pub(crate) folder: Option<String>,
    pub(crate) project_path: Option<String>,
    pub(crate) remote_workspace: Option<CreateRemoteWorkspaceInput>,
    pub(crate) worktree: Option<CreateWorktreeInput>,
}

// `rename_all` renames the *variants* ("minutes", "daily"); it does not reach the fields inside
// them, so without `rename_all_fields` this enum accepted `{kind: "daily", time_of_day: "09:00"}`
// while every caller -- the dialog, `ScheduledTaskFrequency` in types/agent.ts, and the web mock
// client -- sends `timeOfDay`. Daily, weekly and monthly tasks were rejected at the IPC boundary
// with `missing field time_of_day`, and the dialog opens on daily, so the feature's default path
// could not create a task at all. Only minutes and hours worked, their `interval` being one word
// in either convention.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScheduledTaskRun {
    pub(crate) id: String,
    pub(crate) task_id: String,
    pub(crate) session_id: Option<String>,
    pub(crate) status: String,
    pub(crate) error: Option<String>,
    pub(crate) started_at: String,
    pub(crate) completed_at: Option<String>,
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ChatConfig {
    pub(crate) agent_id: String,
    pub(crate) interaction_mode: InteractionMode,
    pub(crate) execution_mode: String,
    #[serde(default)]
    pub(crate) agent_policy: Option<String>,
    #[serde(default)]
    pub(crate) effective_execution_policy: Option<String>,
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
    pub(crate) skill_provenance: Option<SkillToolUseProvenance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillToolUseProvenance {
    pub(crate) skill_id: String,
    pub(crate) tool_id: String,
    pub(crate) revision: String,
    pub(crate) source_scope: String,
    pub(crate) workspace_path: Option<String>,
    pub(crate) redacted_result_summary: Option<String>,
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
    #[serde(default)]
    pub(crate) start_line: Option<u32>,
    #[serde(default)]
    pub(crate) end_line: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) speaker_seat_id: Option<String>,
    /// Index of the seat that spoke this. Absent for user messages and single-Agent sessions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) seat_index: Option<usize>,
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
    pub(crate) session_sequence: u64,
    pub(crate) execution_run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) feedback: Option<MessageFeedback>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MessageFeedback {
    pub(crate) state: Option<String>,
    pub(crate) revision: u64,
    pub(crate) correction_note: Option<String>,
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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TokenUsageSummaryInput {
    pub(crate) session_id: Option<String>,
    pub(crate) message_id: Option<String>,
    pub(crate) generation_id: Option<String>,
    pub(crate) agent_id: Option<String>,
    pub(crate) provider_id: Option<String>,
    pub(crate) model_id: Option<String>,
    pub(crate) purpose: Option<String>,
    pub(crate) quality: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) range_start: Option<String>,
    pub(crate) range_end: Option<String>,
    pub(crate) breakdown_limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TokenDimensions {
    pub(crate) input: i64,
    pub(crate) output: i64,
    pub(crate) cached_input: i64,
    pub(crate) cache_write_input: i64,
    pub(crate) reasoning_output: i64,
    pub(crate) provider_total: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsageMeasure {
    pub(crate) unit: String,
    pub(crate) dimensions: TokenDimensions,
    pub(crate) headline_total: Option<i64>,
    pub(crate) call_count: i64,
    pub(crate) observation_count: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsageQualityTotals {
    pub(crate) reported: UsageMeasure,
    pub(crate) reported_derived: UsageMeasure,
    pub(crate) estimated: UsageMeasure,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsageEntityCounts {
    pub(crate) calls: i64,
    pub(crate) generations: i64,
    pub(crate) sessions: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsageDailyPoint {
    pub(crate) local_date: String,
    pub(crate) totals: UsageQualityTotals,
    pub(crate) counts: UsageEntityCounts,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsageBreakdownEntry {
    pub(crate) key: String,
    pub(crate) totals: UsageQualityTotals,
    pub(crate) counts: UsageEntityCounts,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsageBreakdown {
    pub(crate) dimension: String,
    pub(crate) entries: Vec<UsageBreakdownEntry>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TokenUsageSummary {
    pub(crate) schema_version: u32,
    pub(crate) totals: UsageQualityTotals,
    pub(crate) user_response: UsageQualityTotals,
    pub(crate) internal: UsageQualityTotals,
    pub(crate) counts: UsageEntityCounts,
    pub(crate) daily: Vec<UsageDailyPoint>,
    pub(crate) breakdowns: Vec<UsageBreakdown>,
    pub(crate) generated_at: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TokenUsageDetailsInput {
    pub(crate) session_id: Option<String>,
    pub(crate) agent_id: Option<String>,
    pub(crate) provider_id: Option<String>,
    pub(crate) model_id: Option<String>,
    pub(crate) purpose: Option<String>,
    pub(crate) quality: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) after_id: Option<String>,
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelInvocation {
    pub(crate) id: String,
    pub(crate) generation_id: Option<String>,
    pub(crate) run_id: Option<String>,
    pub(crate) operation_id: Option<String>,
    pub(crate) session_id: String,
    pub(crate) message_id: Option<String>,
    pub(crate) agent_id: String,
    pub(crate) provider_id: Option<String>,
    pub(crate) profile_id: Option<String>,
    pub(crate) endpoint_id: Option<String>,
    pub(crate) model_id: Option<String>,
    pub(crate) interaction_kind: String,
    pub(crate) purpose: String,
    pub(crate) request_sequence: u32,
    pub(crate) attempt: u32,
    pub(crate) status: String,
    pub(crate) started_at: String,
    pub(crate) completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsageObservation {
    pub(crate) id: String,
    pub(crate) invocation_id: String,
    pub(crate) quality: String,
    pub(crate) unit: String,
    pub(crate) measurement_kind: String,
    pub(crate) dimensions: TokenDimensions,
    pub(crate) cache_overlap: String,
    pub(crate) reasoning_overlap: String,
    pub(crate) normalization_version: String,
    pub(crate) source: String,
    pub(crate) source_revision: Option<String>,
    pub(crate) event_at: Option<String>,
    pub(crate) observed_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TokenUsageDetailsPage {
    pub(crate) schema_version: u32,
    pub(crate) invocations: Vec<ModelInvocation>,
    pub(crate) observations: Vec<UsageObservation>,
    pub(crate) next_cursor: Option<String>,
}
