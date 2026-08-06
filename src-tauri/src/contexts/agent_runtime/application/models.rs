use super::loop_models::LoopVerificationCommandView;
use crate::contexts::agent_runtime::domain::{
    AgentAvailability, AgentDefinition, AgentLifecycle, AgentOrigin, AgentReadiness, AgentWorkflow,
    InteractionMode,
};
use crate::contexts::execution_observability::api::ExecutionContext;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
    pub(crate) origin: AgentOrigin,
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
            origin: agent.origin(),
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
    pub(crate) read_only: bool,
    pub(crate) loop_ownership: Option<LoopRoleGenerationOwnership>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopRoleGenerationOwnership {
    pub(crate) run_id: String,
    pub(crate) iteration_id: String,
    pub(crate) role: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopRoleGenerationOutcome {
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopRoleGenerationTerminal {
    pub(crate) run_id: String,
    pub(crate) iteration_id: String,
    pub(crate) role: String,
    pub(crate) session_id: String,
    pub(crate) message_id: String,
    pub(crate) outcome: LoopRoleGenerationOutcome,
    pub(crate) content: Option<String>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct LoopVerificationCancellation {
    cancelled: Arc<AtomicBool>,
}

impl Default for LoopVerificationCancellation {
    fn default() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl LoopVerificationCancellation {
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub(crate) fn signal(&self) -> Arc<AtomicBool> {
        self.cancelled.clone()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LoopVerificationProcessRequest {
    pub(crate) worktree_root: String,
    pub(crate) command: LoopVerificationCommandView,
    pub(crate) cancellation: LoopVerificationCancellation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopVerificationProcessStatus {
    Passed,
    Failed,
    TimedOut,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopVerificationProcessResult {
    pub(crate) status: LoopVerificationProcessStatus,
    pub(crate) exit_code: Option<i32>,
    pub(crate) duration_ms: u64,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) output_truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopOperationKind {
    Worktree,
    RoleGeneration,
    Verification,
    Decision,
    Cancellation,
    Recovery,
}

impl LoopOperationKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Worktree => "worktree",
            Self::RoleGeneration => "role-generation",
            Self::Verification => "verification",
            Self::Decision => "decision",
            Self::Cancellation => "cancellation",
            Self::Recovery => "recovery",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopOperationContext {
    pub(crate) run_id: String,
    pub(crate) iteration_id: Option<String>,
    pub(crate) kind: LoopOperationKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopLog {
    pub(crate) level: AgentLogLevel,
    pub(crate) category: String,
    pub(crate) message: String,
    pub(crate) context: LoopOperationContext,
    pub(crate) operation_id: Option<String>,
    pub(crate) occurred_at: String,
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

/// A tool the native tool-use loop can declare to a provider. Provider-agnostic — each wire
/// format translation module renders this into its own `tools` request shape.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ToolDefinition {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) input_schema: Value,
}

/// The result of invoking a dynamically-sourced tool (currently: MCP-sourced tools only) through
/// `AgentMcpToolPort`. Kept separate from infrastructure's `ToolExecutionOutcome` — an
/// application-layer port trait cannot mention an infrastructure-layer type in its signature
/// (`native_context_dependencies_point_inward` forbids `application` from importing this
/// context's own `infrastructure`, not just another context's) — `execute_tool_call` converts
/// one into the other field-for-field at the infrastructure call site.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AgentToolCallOutcome {
    pub(crate) output: String,
    pub(crate) is_error: bool,
}

/// Whether a tool call may execute immediately or must wait for an explicit user decision.
/// Fixed per tool/operation (design.md Decision 4) — never derived from the call's arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolRiskTier {
    AutoApprove,
    RequiresApproval,
}

/// The user's resolution of a tool call that was awaiting approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolApprovalDecision {
    Approved,
    Denied,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentUsageAccountingKind {
    Reported,
    Estimated,
}

/// Reported token usage normalized to the application layer's own shape — kept
/// separate from `agent_runtime::infrastructure::ProviderReportedUsage` (the raw
/// per-CLI shape) so this layer never depends on an infrastructure-defined type.
/// See `add-reported-usage-ingestion` design.md Decision 0/2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ReportedUsageTotals {
    pub(crate) input_tokens: i64,
    pub(crate) output_tokens: i64,
    pub(crate) cache_read_tokens: i64,
    pub(crate) cache_creation_tokens: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentUsageRecord {
    pub(crate) message_id: String,
    pub(crate) session_id: String,
    pub(crate) agent_id: String,
    pub(crate) provider_id: Option<String>,
    pub(crate) model_id: Option<String>,
    pub(crate) accounting_kind: AgentUsageAccountingKind,
    pub(crate) input_count: i64,
    pub(crate) output_count: i64,
    pub(crate) cache_read_count: i64,
    pub(crate) cache_creation_count: i64,
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
    pub(crate) version: Option<i64>,
    pub(crate) content_hash: Option<String>,
    pub(crate) token_estimate: Option<usize>,
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptVersionReference {
    pub(crate) hook_id: String,
    pub(crate) version: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PromptExecutionOutcome {
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptExecutionReport {
    pub(crate) invocation_id: String,
    pub(crate) agent_id: String,
    pub(crate) versions: Vec<PromptVersionReference>,
    pub(crate) outcome: PromptExecutionOutcome,
    pub(crate) elapsed_ms: i64,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingPromptExecution {
    pub(crate) invocation_id: String,
    pub(crate) agent_id: String,
    pub(crate) versions: Vec<PromptVersionReference>,
    pub(crate) started_at: std::time::Instant,
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
    pub(crate) execution_context: ExecutionContext,
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
    // Kept for compatibility with legacy event sinks while providers migrate to lifecycle events.
    #[allow(dead_code)]
    ToolUse(ToolUseBlock),
    ToolLifecycle(ToolLifecycleEvent),
    RichBlock(Value),
    RuntimeSessionId(String),
    Stderr(String),
    Completed(Option<ReportedUsageTotals>),
    Failed(GenerationProcessFailure),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationProcessFailureKind {
    Retryable,
    NonRetryable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenerationProcessFailure {
    pub(crate) kind: GenerationProcessFailureKind,
    pub(crate) diagnostic: String,
    pub(crate) safe_error: Option<String>,
}

impl GenerationProcessFailure {
    pub(crate) fn retryable(diagnostic: impl Into<String>) -> Self {
        Self {
            kind: GenerationProcessFailureKind::Retryable,
            diagnostic: diagnostic.into(),
            safe_error: None,
        }
    }

    pub(crate) fn non_retryable(diagnostic: impl Into<String>) -> Self {
        Self {
            kind: GenerationProcessFailureKind::NonRetryable,
            diagnostic: diagnostic.into(),
            safe_error: None,
        }
    }

    pub(crate) fn with_safe_error(mut self, safe_error: impl Into<String>) -> Self {
        self.safe_error = Some(safe_error.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolLifecyclePhase {
    /// A tool call whose risk tier requires user approval is waiting on that decision before it
    /// executes. CLI-agent stdout parsing never produces this phase — it is only ever emitted by
    /// the native tool-use loop.
    AwaitingApproval,
    Started,
    Updated,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ToolLifecycleEvent {
    pub(crate) call_id: String,
    pub(crate) phase: ToolLifecyclePhase,
    pub(crate) provider_timestamp: Option<String>,
    pub(crate) fidelity: crate::contexts::execution_observability::api::ExecutionFidelity,
    pub(crate) parent_run_id: Option<String>,
    pub(crate) parent_trace_id: Option<String>,
    pub(crate) parent_span_id: Option<String>,
    pub(crate) delegation_id: Option<String>,
    pub(crate) attempt: Option<u32>,
    pub(crate) tool_use: ToolUseBlock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartedGenerationProcess {
    pub(crate) process_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessStopInitiator {
    User,
    RuntimeCleanup,
}

impl ProcessStopInitiator {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::RuntimeCleanup => "runtime_cleanup",
        }
    }
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
    pub(crate) execution_context: Option<ExecutionContext>,
    pub(crate) prompt_execution: Option<PendingPromptExecution>,
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
    pub(crate) run_id: Option<String>,
    pub(crate) trace_id: Option<String>,
    pub(crate) span_id: Option<String>,
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
    pub(crate) source: AgentMessageSource,
    pub(crate) session_id: String,
    pub(crate) content: String,
    pub(crate) configuration: AgentChatConfiguration,
    pub(crate) file_references: Vec<AgentFileReference>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentMessageSource {
    Desktop,
    InstantMessage { connector_id: String },
    Scheduled { task_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentMessageTerminalOutcome {
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentMessageTerminal {
    pub(crate) session_id: String,
    pub(crate) message_id: String,
    pub(crate) outcome: AgentMessageTerminalOutcome,
    pub(crate) content: Option<String>,
}

pub(crate) struct AgentMessageTerminalReceiver {
    receiver: std::sync::mpsc::Receiver<AgentMessageTerminal>,
    cleanup: Option<Box<dyn FnOnce() + Send>>,
}

impl AgentMessageTerminalReceiver {
    pub(crate) fn new(
        receiver: std::sync::mpsc::Receiver<AgentMessageTerminal>,
        cleanup: Box<dyn FnOnce() + Send>,
    ) -> Self {
        Self {
            receiver,
            cleanup: Some(cleanup),
        }
    }

    pub(crate) fn recv_timeout(
        self,
        timeout: std::time::Duration,
    ) -> Result<AgentMessageTerminal, std::sync::mpsc::RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }
}

impl Drop for AgentMessageTerminalReceiver {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup();
        }
    }
}

pub(crate) struct StartedAgentMessage {
    pub(crate) message: AgentMessage,
    pub(crate) terminal: AgentMessageTerminalReceiver,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StopGenerationResult {
    pub(crate) cancelled_message_ids: Vec<String>,
    pub(crate) process_stopped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegisterApiAgentInput {
    pub(crate) display_name: String,
    pub(crate) provider: String,
    pub(crate) api_key: String,
    pub(crate) model_id: String,
    pub(crate) interface_format: String,
    pub(crate) base_url: Option<String>,
}

/// `add-agent-lifecycle-management`. `provider`/`interface_format` are deliberately absent —
/// immutable after registration (design.md Decision 1). `new_api_key: None` leaves the stored
/// credential untouched; `ApiAgentGateway::update` never reads this field — only the
/// application service does, exactly like `RegisterApiAgentInput.api_key` is only ever read by
/// the service before calling `register`, never by the gateway itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UpdateApiAgentInput {
    pub(crate) display_name: String,
    pub(crate) model_id: String,
    pub(crate) base_url: Option<String>,
    pub(crate) new_api_key: Option<String>,
}

/// The two supported wire protocols for `launch_kind = "api"` agents.
pub(crate) const INTERFACE_FORMAT_ANTHROPIC: &str = "anthropic";
pub(crate) const INTERFACE_FORMAT_OPENAI_COMPATIBLE: &str = "openai-compatible";

/// Per-agent configuration `RuntimeAgentApiAdapter::execute` needs to run a generation:
/// which model, which wire protocol, and (for `openai-compatible`) which endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApiProviderConfig {
    pub(crate) model_id: String,
    pub(crate) interface_format: String,
    pub(crate) base_url: Option<String>,
    pub(crate) auto_approve_tools: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OnePieceProviderConfig {
    pub(crate) provider: String,
    pub(crate) model_id: Option<String>,
    pub(crate) interface_format: Option<String>,
    pub(crate) base_url: Option<String>,
    pub(crate) auto_approve_tools: bool,
    pub(crate) credential_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SaveOnePieceProviderConfigInput {
    pub(crate) provider: String,
    pub(crate) model_id: String,
    pub(crate) interface_format: String,
    pub(crate) base_url: Option<String>,
    pub(crate) api_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredOnePieceProviderConfig {
    pub(crate) provider: String,
    pub(crate) model_id: Option<String>,
    pub(crate) interface_format: Option<String>,
    pub(crate) base_url: Option<String>,
    pub(crate) auto_approve_tools: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub(crate) model_discovery_strategy: String,
    pub(crate) default_endpoint_type: String,
    pub(crate) endpoints: Vec<OnePieceProviderEndpoint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OnePieceProviderEndpoint {
    pub(crate) endpoint_type: String,
    pub(crate) base_url: String,
    pub(crate) interface_format: String,
    pub(crate) auth_strategy: String,
    pub(crate) source: String,
    pub(crate) model_discovery_strategy: String,
    pub(crate) model_discovery_url: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct DiscoverOnePieceProviderModelsInput {
    pub(crate) provider_id: String,
    pub(crate) endpoint_type: String,
    pub(crate) profile_id: Option<String>,
    pub(crate) api_key: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ValidateOnePieceProviderCredentialInput {
    pub(crate) provider_id: String,
    pub(crate) endpoint_type: String,
    pub(crate) model_id: String,
    pub(crate) profile_id: Option<String>,
    pub(crate) api_key: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderCredentialProbeProtocol {
    AnthropicMessages,
    OpenAiChatCompletions,
    OpenAiResponses,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderCredentialProbeAuthentication {
    AnthropicApiKey,
    Bearer,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ProviderCredentialValidationStatus {
    Valid,
    InvalidCredential,
    ConfigurationRejected,
    RateLimited,
    ProviderUnavailable,
    Unsupported,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderCredentialValidationResult {
    pub(crate) status: ProviderCredentialValidationStatus,
    pub(crate) latency_ms: u64,
    pub(crate) http_status: Option<u16>,
}

pub(crate) struct ProviderCredentialProbeRequest {
    pub(crate) base_url: String,
    pub(crate) model: String,
    pub(crate) protocol: ProviderCredentialProbeProtocol,
    pub(crate) authentication: ProviderCredentialProbeAuthentication,
    pub(crate) credential: String,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OnePieceModelDiscoveryRequest {
    pub(crate) strategy: String,
    pub(crate) url: String,
    pub(crate) api_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OnePieceDiscoveredModel {
    pub(crate) id: String,
    pub(crate) display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OnePieceProviderModelOption {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OnePieceProviderModelDiscoveryResult {
    pub(crate) provider_id: String,
    pub(crate) endpoint_type: String,
    pub(crate) models: Vec<OnePieceProviderModelOption>,
    pub(crate) source: String,
    pub(crate) warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OnePieceProviderProfiles {
    pub(crate) profiles: Vec<OnePieceProviderProfile>,
    pub(crate) active_profile_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SaveOnePieceProviderProfileInput {
    pub(crate) id: Option<String>,
    pub(crate) name: String,
    pub(crate) provider_id: String,
    pub(crate) endpoint_type: String,
    pub(crate) model_id: String,
    pub(crate) api_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredOnePieceProviderProfile {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) source_preset_id: Option<String>,
    pub(crate) source_provider_id: Option<String>,
    pub(crate) source_endpoint_type: Option<String>,
    pub(crate) source_preset_version: Option<u32>,
    pub(crate) provider: String,
    pub(crate) model_id: String,
    pub(crate) interface_format: String,
    pub(crate) base_url: Option<String>,
    pub(crate) active: bool,
}

// 唯一构造点是 resolve_embedding_endpoint；该方法要到 Task 12 的 bootstrap 适配器调用它（实现
// retrieval::EmbeddingEndpointPort::resolve）才可达。届时移除本属性。凭据是原始字符串，故意不
// 派生 Debug——避免某处 `{:?}` 意外把它写进日志或错误消息，呼应 OnePieceModelDiscoveryRequest
// （同样携带 api_key）不派生 Debug 的既有先例。
#[allow(dead_code)]
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct EmbeddingEndpointView {
    pub(crate) base_url: String,
    pub(crate) interface_format: String,
    pub(crate) credential: String,
}

/// A Skill bound to an API agent, resolved and ready to inject as that agent's generation
/// requests' system prompt (`add-agent-skill-support`) — `name` and `body` only, no metadata
/// `agent_runtime` has no use for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BoundSkillPrompt {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentCoreInstructions {
    pub(crate) version: String,
    pub(crate) markdown: String,
}

/// How a memory (`add-agent-cross-session-memory`) was produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemorySource {
    /// Saved by the model calling the `remember` tool.
    Explicit,
    /// Saved by best-effort extraction when context compaction triggers.
    Automatic,
}

impl MemorySource {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::Automatic => "automatic",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "explicit" => Some(Self::Explicit),
            "automatic" => Some(Self::Automatic),
            _ => None,
        }
    }
}

/// A persisted cross-session memory, scoped to the agent that produced it and (when available)
/// its workspace folder — `folder: None` means the agent-global scope (`add-agent-cross-session-memory`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentMemory {
    pub(crate) id: String,
    pub(crate) agent_id: String,
    pub(crate) folder: Option<String>,
    pub(crate) content: String,
    pub(crate) source: MemorySource,
    pub(crate) created_at: String,
}
