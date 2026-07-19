use crate::contexts::agent_runtime::domain::{
    AgentAvailability, AgentDefinition, AgentLifecycle, AgentReadiness, AgentWorkflow,
    InteractionMode,
};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentLaunchView {
    pub(crate) kind: String,
    pub(crate) command: Option<String>,
    pub(crate) url: Option<String>,
    pub(crate) executable_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentView {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) provider: String,
    pub(crate) managed_sdk_dependency_id: Option<String>,
    pub(crate) launch: AgentLaunchView,
    pub(crate) supported_interaction_modes: Vec<InteractionMode>,
    pub(crate) availability: AgentAvailability,
    pub(crate) unavailable_reason: Option<String>,
    pub(crate) capability_tags: Vec<String>,
}

impl From<&AgentDefinition> for AgentView {
    fn from(agent: &AgentDefinition) -> Self {
        Self {
            id: agent.id().as_str().to_string(),
            display_name: agent.display_name().to_string(),
            provider: agent.provider().to_string(),
            managed_sdk_dependency_id: agent.managed_sdk_dependency_id().map(str::to_string),
            launch: AgentLaunchView {
                kind: agent.launch().kind_str().to_string(),
                command: agent.launch().command().map(str::to_string),
                url: agent.launch().url().map(str::to_string),
                executable_name: agent.launch().executable_name().map(str::to_string),
            },
            supported_interaction_modes: agent.supported_interaction_modes().to_vec(),
            availability: agent.availability().state(),
            unavailable_reason: agent.availability().reason().map(str::to_string),
            capability_tags: agent.capability_tags().to_vec(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowView {
    pub(crate) active_agent_id: Option<String>,
    pub(crate) active_interaction_mode: Option<InteractionMode>,
    pub(crate) lifecycle: AgentLifecycle,
    pub(crate) intent: String,
}

impl From<&AgentWorkflow> for WorkflowView {
    fn from(workflow: &AgentWorkflow) -> Self {
        Self {
            active_agent_id: workflow
                .active_agent_id()
                .map(|agent_id| agent_id.as_str().to_string()),
            active_interaction_mode: workflow.active_interaction_mode(),
            lifecycle: workflow.lifecycle(),
            intent: workflow.intent().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReadinessView {
    pub(crate) ready: bool,
    pub(crate) reason: Option<String>,
    pub(crate) requires_authentication: bool,
}

impl From<AgentReadiness> for ReadinessView {
    fn from(readiness: AgentReadiness) -> Self {
        Self {
            ready: readiness.is_ready(),
            reason: readiness.reason().map(str::to_string),
            requires_authentication: readiness.requires_authentication(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaunchWorkflowResult {
    pub(crate) operation_id: String,
    pub(crate) workflow: WorkflowView,
    pub(crate) message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentSessionDetails {
    pub(crate) workflow: WorkflowView,
    pub(crate) adapter: String,
    pub(crate) details: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentSession {
    pub(crate) id: String,
    pub(crate) agent_id: String,
    pub(crate) interaction_mode: InteractionMode,
    pub(crate) lifecycle: AgentLifecycle,
    pub(crate) folder: Option<String>,
    pub(crate) runtime_session_id: Option<String>,
    pub(crate) archived: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentTerminalState {
    #[expect(
        dead_code,
        reason = "reserved for future asynchronous terminal startup events"
    )]
    Starting,
    Running,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentTerminalCapability {
    Native,
    #[expect(
        dead_code,
        reason = "frontend contract includes simulated terminals for web/mock parity"
    )]
    Simulated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentTerminalSize {
    pub(crate) rows: u16,
    pub(crate) cols: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenAgentTerminalRequest {
    pub(crate) session_id: String,
    pub(crate) size: AgentTerminalSize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentTerminalInputRequest {
    pub(crate) terminal_id: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResizeAgentTerminalRequest {
    pub(crate) terminal_id: String,
    pub(crate) size: AgentTerminalSize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StopAgentTerminalRequest {
    pub(crate) terminal_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AgentTerminalProcessRequest {
    pub(crate) session: AgentSession,
    pub(crate) agent: AgentView,
    pub(crate) cli_profile: CliProfileSnapshot,
    pub(crate) size: AgentTerminalSize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentTerminalEvent {
    Output {
        terminal_id: String,
        session_id: String,
        content: String,
    },
    State {
        terminal_id: String,
        session_id: String,
        state: AgentTerminalState,
        error: Option<String>,
    },
    RuntimeSessionId {
        terminal_id: String,
        session_id: String,
        runtime_session_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentChatConfiguration {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentFileReference {
    pub(crate) id: String,
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) size_bytes: Option<i64>,
    pub(crate) content_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ToolUseBlock {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) input: Option<Value>,
    pub(crate) output: Option<Value>,
    pub(crate) status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MessageTokenUsage {
    pub(crate) input: i64,
    pub(crate) output: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AgentMessage {
    pub(crate) id: String,
    pub(crate) session_id: String,
    pub(crate) role: String,
    pub(crate) content: String,
    pub(crate) status: String,
    pub(crate) tool_use: Vec<ToolUseBlock>,
    pub(crate) thinking_content: Option<String>,
    pub(crate) rich_blocks: Vec<Value>,
    pub(crate) token_usage: Option<MessageTokenUsage>,
    pub(crate) file_references: Vec<AgentFileReference>,
    pub(crate) error: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewAgentMessage {
    pub(crate) session_id: String,
    pub(crate) role: String,
    pub(crate) status: String,
    pub(crate) content: String,
    pub(crate) file_references: Vec<AgentFileReference>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentUsageRecord {
    pub(crate) message_id: String,
    pub(crate) session_id: String,
    pub(crate) agent_id: String,
    pub(crate) provider_id: Option<String>,
    pub(crate) model_id: Option<String>,
    pub(crate) input_count: i64,
    pub(crate) output_count: i64,
    pub(crate) source: String,
    pub(crate) occurred_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompleteAgentMessage {
    pub(crate) message_id: String,
    pub(crate) session_id: String,
    pub(crate) content: String,
    pub(crate) thinking_content: Option<String>,
    pub(crate) tool_use: Vec<ToolUseBlock>,
    pub(crate) rich_blocks: Vec<Value>,
    pub(crate) token_usage: Option<MessageTokenUsage>,
    pub(crate) usage: Option<AgentUsageRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptTrace {
    pub(crate) hook_id: String,
    pub(crate) status: String,
    pub(crate) content_hash: Option<String>,
    pub(crate) token_estimate: Option<usize>,
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EffectivePrompt {
    pub(crate) content: String,
    pub(crate) trace: Vec<PromptTrace>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CliProfileSnapshot {
    pub(crate) executable: String,
    pub(crate) selections: BTreeMap<String, Value>,
    pub(crate) managed_args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowLaunchRequest {
    pub(crate) operation_id: String,
    pub(crate) agent: AgentView,
    pub(crate) interaction_mode: InteractionMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowLaunchOutcome {
    pub(crate) adapter: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GenerationProcessRequest {
    pub(crate) session: AgentSession,
    pub(crate) agent: AgentView,
    pub(crate) message_id: String,
    pub(crate) operation_id: String,
    pub(crate) configuration: AgentChatConfiguration,
    pub(crate) effective_prompt: String,
    pub(crate) cli_profile: CliProfileSnapshot,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GenerationProcessEvent {
    Token(String),
    Thinking(String),
    ToolUse(ToolUseBlock),
    RichBlock(Value),
    RuntimeSessionId(String),
    Stderr(String),
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartedGenerationProcess {
    pub(crate) process_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenerationLease {
    pub(crate) session_id: String,
    pub(crate) lease_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenerationCancellation {
    pub(crate) message_id: Option<String>,
    pub(crate) process_id: Option<String>,
    pub(crate) operation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentOperation {
    pub(crate) id: String,
    pub(crate) related_agent_id: Option<String>,
    pub(crate) message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentLogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentLog {
    pub(crate) level: AgentLogLevel,
    pub(crate) category: String,
    pub(crate) message: String,
    pub(crate) agent_id: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) operation_id: Option<String>,
    pub(crate) occurred_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AgentEvent {
    WorkflowChanged(WorkflowView),
    MessageStarted {
        session_id: String,
        message_id: String,
    },
    MessageToken {
        session_id: String,
        message_id: String,
        content_delta: String,
    },
    MessageThinking {
        session_id: String,
        message_id: String,
        content_delta: String,
    },
    MessageToolUse {
        session_id: String,
        message_id: String,
        tool_use: ToolUseBlock,
    },
    MessageRichBlock {
        session_id: String,
        message_id: String,
        block: Value,
    },
    MessageCompleted {
        session_id: String,
        message_id: String,
        token_usage: Option<MessageTokenUsage>,
    },
    MessageFailed {
        session_id: String,
        message_id: String,
        error: String,
    },
    MessageCancelled {
        session_id: String,
        message_id: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SendMessageRequest {
    pub(crate) session_id: String,
    pub(crate) content: String,
    pub(crate) configuration: AgentChatConfiguration,
    pub(crate) file_references: Vec<AgentFileReference>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StopGenerationResult {
    pub(crate) cancelled_message_ids: Vec<String>,
    pub(crate) process_stopped: bool,
}
