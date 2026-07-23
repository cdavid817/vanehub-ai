use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum InteractionMode {
    Browser,
    NativeDesktop,
    Cli,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AvailabilityState {
    Available,
    Unavailable,
    NeedsAuth,
    Unknown,
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
pub(crate) struct LaunchMetadata {
    pub(crate) kind: String,
    pub(crate) command: Option<String>,
    pub(crate) url: Option<String>,
    pub(crate) executable_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentRegistryEntry {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) provider: String,
    pub(crate) managed_sdk_dependency_id: Option<String>,
    pub(crate) launch: LaunchMetadata,
    pub(crate) supported_interaction_modes: Vec<InteractionMode>,
    pub(crate) availability_state: AvailabilityState,
    pub(crate) unavailable_reason: Option<String>,
    pub(crate) capability_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowState {
    pub(crate) active_agent_id: Option<String>,
    pub(crate) active_interaction_mode: Option<InteractionMode>,
    pub(crate) lifecycle_state: SessionLifecycleState,
    pub(crate) intent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReadinessStatus {
    pub(crate) ready: bool,
    pub(crate) reason: Option<String>,
    pub(crate) requires_authentication: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LaunchResult {
    pub(crate) operation_id: Option<String>,
    pub(crate) workflow: WorkflowState,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionDetails {
    pub(crate) agent_id: Option<String>,
    pub(crate) interaction_mode: Option<InteractionMode>,
    pub(crate) lifecycle_state: SessionLifecycleState,
    pub(crate) adapter: String,
    pub(crate) details: std::collections::BTreeMap<String, String>,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AgentTerminalState {
    Starting,
    Running,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AgentTerminalCapability {
    Native,
    Simulated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentTerminalSize {
    pub(crate) rows: u16,
    pub(crate) cols: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentTerminalSession {
    pub(crate) terminal_id: String,
    pub(crate) session_id: String,
    pub(crate) agent_id: String,
    pub(crate) state: AgentTerminalState,
    pub(crate) capability: AgentTerminalCapability,
    pub(crate) size: AgentTerminalSize,
    pub(crate) runtime_session_id: Option<String>,
    pub(crate) retained: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoordinationNodeInput {
    pub(crate) id: String,
    pub(crate) primary_agent_id: String,
    #[serde(default)]
    pub(crate) fallback_agent_ids: Vec<String>,
    pub(crate) instruction: String,
    #[serde(default)]
    pub(crate) depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartCoordinationInput {
    pub(crate) name: String,
    pub(crate) project_path: Option<String>,
    pub(crate) nodes: Vec<CoordinationNodeInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartCoordinationResult {
    pub(crate) run_id: String,
    pub(crate) operation_id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CoordinationRunStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CoordinationNodeStatus {
    Blocked,
    Queued,
    Running,
    Succeeded,
    Failed,
    Skipped,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CoordinationAttemptStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CoordinationFailureKind {
    Retryable,
    NonRetryable,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CoordinationCandidateRole {
    Primary,
    Fallback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoordinationOutput {
    pub(crate) source_node_id: String,
    pub(crate) agent_id: String,
    pub(crate) attempt: u32,
    pub(crate) content: String,
    pub(crate) byte_count: usize,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoordinationAttempt {
    pub(crate) attempt: u32,
    pub(crate) agent_id: String,
    pub(crate) candidate_role: CoordinationCandidateRole,
    pub(crate) status: CoordinationAttemptStatus,
    pub(crate) failure_kind: Option<CoordinationFailureKind>,
    pub(crate) error: Option<String>,
    pub(crate) started_at: String,
    pub(crate) completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoordinationNodeRun {
    pub(crate) id: String,
    pub(crate) primary_agent_id: String,
    pub(crate) fallback_agent_ids: Vec<String>,
    pub(crate) instruction: String,
    pub(crate) depends_on: Vec<String>,
    pub(crate) status: CoordinationNodeStatus,
    pub(crate) actual_agent_id: Option<String>,
    pub(crate) output: Option<CoordinationOutput>,
    pub(crate) attempts: Vec<CoordinationAttempt>,
    pub(crate) error: Option<String>,
    pub(crate) started_at: Option<String>,
    pub(crate) completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoordinationRun {
    pub(crate) id: String,
    pub(crate) operation_id: String,
    pub(crate) name: String,
    pub(crate) project_path: Option<String>,
    pub(crate) status: CoordinationRunStatus,
    pub(crate) nodes: Vec<CoordinationNodeRun>,
    pub(crate) simulated: bool,
    pub(crate) cancel_requested: bool,
    pub(crate) created_at: String,
    pub(crate) started_at: Option<String>,
    pub(crate) updated_at: String,
    pub(crate) completed_at: Option<String>,
}
