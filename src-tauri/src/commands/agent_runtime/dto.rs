//! Tauri transport DTOs for Agent Runtime commands.
//!
//! These serializable types isolate camel-case frontend contracts from native domain and
//! application models. Mapping is explicit so command compatibility does not leak transport
//! concerns into the owning bounded context.

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
pub(crate) enum AgentOrigin {
    Builtin,
    User,
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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegisterApiAgentInput {
    pub(crate) display_name: String,
    pub(crate) provider: String,
    pub(crate) api_key: String,
    pub(crate) model_id: String,
    pub(crate) interface_format: String,
    pub(crate) base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApiAgentProviderConfig {
    pub(crate) model_id: String,
    pub(crate) interface_format: String,
    pub(crate) base_url: Option<String>,
    pub(crate) auto_approve_tools: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateApiAgentInput {
    pub(crate) display_name: String,
    pub(crate) model_id: String,
    pub(crate) base_url: Option<String>,
    pub(crate) new_api_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolveToolApprovalInput {
    pub(crate) session_id: String,
    pub(crate) call_id: String,
    pub(crate) approved: bool,
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
    pub(crate) agent_origin: AgentOrigin,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OnePieceProviderConfig {
    pub(crate) provider: String,
    pub(crate) model_id: Option<String>,
    pub(crate) interface_format: Option<String>,
    pub(crate) base_url: Option<String>,
    pub(crate) auto_approve_tools: bool,
    pub(crate) credential_present: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveOnePieceProviderConfigInput {
    pub(crate) provider: String,
    pub(crate) model_id: String,
    pub(crate) interface_format: String,
    pub(crate) base_url: Option<String>,
    pub(crate) api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OnePieceProviderProfile {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) source_provider_id: Option<String>,
    pub(crate) source_endpoint_type: Option<String>,
    pub(crate) source_preset_version: Option<u32>,
    pub(crate) provider: String,
    pub(crate) model_id: String,
    pub(crate) interface_format: String,
    pub(crate) base_url: Option<String>,
    pub(crate) active: bool,
    pub(crate) credential_present: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OnePieceProviderPreset {
    pub(crate) id: String,
    pub(crate) catalog_version: u32,
    pub(crate) display_name: String,
    pub(crate) category: String,
    pub(crate) icon_key: String,
    pub(crate) provider: String,
    pub(crate) default_model_id: String,
    pub(crate) fallback_models: Vec<String>,
    pub(crate) interface_format: String,
    pub(crate) base_url: Option<String>,
    pub(crate) api_key_url: String,
    pub(crate) docs_url: String,
    pub(crate) model_discovery: OnePieceModelDiscoveryMetadata,
    pub(crate) default_endpoint_type: String,
    pub(crate) endpoints: Vec<OnePieceProviderEndpoint>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OnePieceProviderEndpoint {
    #[serde(rename = "type")]
    pub(crate) endpoint_type: String,
    pub(crate) base_url: String,
    pub(crate) interface_format: String,
    pub(crate) auth_strategy: String,
    pub(crate) source: String,
    pub(crate) model_discovery: OnePieceEndpointDiscoveryMetadata,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OnePieceEndpointDiscoveryMetadata {
    pub(crate) strategy: String,
    pub(crate) url: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OnePieceModelDiscoveryMetadata {
    pub(crate) strategy: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiscoverOnePieceProviderModelsInput {
    pub(crate) provider_id: String,
    pub(crate) endpoint_type: String,
    pub(crate) profile_id: Option<String>,
    pub(crate) api_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ValidateOnePieceProviderCredentialInput {
    pub(crate) provider_id: String,
    pub(crate) endpoint_type: String,
    pub(crate) model_id: String,
    pub(crate) profile_id: Option<String>,
    pub(crate) api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OnePieceProviderModelOption {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) source: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OnePieceProviderModelDiscoveryResult {
    pub(crate) provider_id: String,
    pub(crate) endpoint_type: String,
    pub(crate) models: Vec<OnePieceProviderModelOption>,
    pub(crate) source: String,
    pub(crate) warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OnePieceProviderProfiles {
    pub(crate) profiles: Vec<OnePieceProviderProfile>,
    pub(crate) active_profile_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveOnePieceProviderProfileInput {
    pub(crate) id: Option<String>,
    pub(crate) name: String,
    pub(crate) provider_id: String,
    pub(crate) endpoint_type: String,
    pub(crate) model_id: String,
    pub(crate) api_key: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AgentMemorySource {
    Explicit,
    Automatic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentMemoryEntry {
    pub(crate) id: String,
    pub(crate) agent_id: String,
    pub(crate) folder: Option<String>,
    pub(crate) content: String,
    pub(crate) source: AgentMemorySource,
    pub(crate) created_at: String,
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
pub(crate) struct ExpertRoleReviewPolicy {
    pub(crate) peer_reviewer: bool,
    pub(crate) require_different_family: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// Page-facing projection of a reusable expert role.
pub(crate) struct ExpertRole {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) avatar: String,
    pub(crate) color: String,
    pub(crate) responsibility: String,
    pub(crate) instruction: String,
    pub(crate) skill_ids: Vec<String>,
    pub(crate) review_policy: ExpertRoleReviewPolicy,
    pub(crate) preferred_providers: Vec<String>,
    pub(crate) origin: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveExpertRoleInput {
    pub(crate) id: Option<String>,
    pub(crate) display_name: String,
    pub(crate) avatar: String,
    pub(crate) color: String,
    pub(crate) responsibility: String,
    pub(crate) instruction: String,
    pub(crate) skill_ids: Vec<String>,
    pub(crate) review_policy: ExpertRoleReviewPolicy,
    pub(crate) preferred_providers: Vec<String>,
}
