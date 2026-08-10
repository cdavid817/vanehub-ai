mod error;
mod expert_role;
mod loop_control;
mod loop_models;
mod loop_observability;
mod loop_orchestrator;
mod loop_orchestrator_decision;
mod loop_orchestrator_support;
mod loop_progress;
mod loop_recovery;
mod loop_service;
mod loop_verification;
mod loop_verifier;
mod loop_worker;
mod loop_worker_prompt;
mod model_category;
mod models;
mod onepiece_provider_catalog;
mod ports;
mod provider;
mod seat_turn;
#[cfg(test)]
mod seat_turn_tests;
mod service;
mod terminal_service;
mod tool_catalog;

pub(crate) use crate::contexts::agent_runtime::domain::LoopVerifierRecommendation;
pub(crate) use error::AgentRuntimeApplicationError;
pub(crate) use expert_role::{
    ExpertRoleApplicationPorts, ExpertRoleApplicationService, ExpertRoleClockPort,
    ExpertRoleIdPort, ExpertRolePort,
};
pub(crate) use loop_control::{LoopControlApplicationPorts, LoopControlApplicationService};
#[cfg(test)]
pub(crate) use loop_models::LoopLimitsView;
pub(crate) use loop_models::LoopVerificationCommandView;
pub(crate) use loop_models::{
    ContinueLoopRequest, LoopChildRecoveryDecision, LoopChildRecoveryProjection,
    LoopDefinitionView, LoopEvidenceView, LoopGitStateEntryView, LoopGitStateView,
    LoopIterationView, LoopOwnedRecoverySession, LoopRoleSessionRequest, LoopRunView,
    LoopVerificationBatchResult, LoopVerifierResult, PreparedLoopWorktree,
    RunLoopVerificationRequest, SaveLoopDefinitionRequest, SaveLoopVerifierResultRequest,
    StartLoopResultView, StartLoopVerifierRequest, StartLoopWorkerRequest, StartedLoopVerifierView,
    StartedLoopWorkerView,
};
pub(crate) use loop_observability::{ActiveLoopOperation, LoopOperationObserver};
pub(crate) use loop_orchestrator::{LoopOrchestratorApplicationService, LoopOrchestratorPorts};
#[cfg(test)]
pub(crate) use loop_progress::fingerprint_loop_iteration;
pub(crate) use loop_progress::{LoopProgressApplicationService, RecordLoopRevisionProgressRequest};
pub(crate) use loop_recovery::{LoopRecoveryApplicationPorts, LoopRecoveryApplicationService};
pub(crate) use loop_service::{LoopApplicationPorts, LoopApplicationService};
pub(crate) use loop_verification::{
    LoopVerificationApplicationPorts, LoopVerificationApplicationService,
};
pub(crate) use loop_verifier::{LoopVerifierApplicationPorts, LoopVerifierApplicationService};
pub(crate) use loop_worker::{LoopWorkerApplicationPorts, LoopWorkerApplicationService};
#[cfg(test)]
pub(crate) use models::AgentLaunchView;
pub(crate) use models::{
    format_memory_section, ActiveGenerationCorrelation, AgentChatConfiguration,
    AgentCoreInstructions, AgentEvent, AgentFileReference, AgentLog, AgentLogLevel, AgentMemory,
    AgentMessage, AgentMessageSource, AgentMessageTerminal, AgentMessageTerminalOutcome,
    AgentMessageTerminalReceiver, AgentOperation, AgentSession, AgentSessionDetails,
    AgentSessionSeat, AgentTerminalCapability, AgentTerminalEvent, AgentTerminalInputRequest,
    AgentTerminalProcessRequest, AgentTerminalSession, AgentTerminalSize, AgentTerminalState,
    AgentToolCallOutcome, AgentUsageAccountingKind, AgentUsageRecord, AgentView, ApiProviderConfig,
    BoundSkillPrompt, CliProfileSnapshot, CompleteAgentMessage,
    DiscoverOnePieceProviderModelsInput, DurableAgentGenerationMessages,
    DurableAgentGenerationStart, EffectivePrompt, EmbeddingEndpointView, ExecutionToolMode,
    GenerationCancellation, GenerationLease, GenerationProcessEvent, GenerationProcessFailure,
    GenerationProcessRequest, LaunchWorkflowResult, LoopLog, LoopOperationContext,
    LoopOperationKind, LoopRoleGenerationOutcome, LoopRoleGenerationOwnership,
    LoopRoleGenerationTerminal, LoopVerificationCancellation, LoopVerificationProcessRequest,
    LoopVerificationProcessResult, LoopVerificationProcessStatus, MemorySource, MessageTokenUsage,
    NewAgentMessage, OnePieceDiscoveredModel, OnePieceModelDiscoveryRequest,
    OnePiecePlanningRequest, OnePiecePlanningResult, OnePieceProviderConfig,
    OnePieceProviderEndpoint, OnePieceProviderModelDiscoveryResult, OnePieceProviderModelOption,
    OnePieceProviderPreset, OnePieceProviderProfile, OnePieceProviderProfiles,
    OpenAgentTerminalRequest, OrchestrationCorrelation, OrchestrationExecutionProfile,
    PendingPromptExecution, PersonalizationSettings, ProcessStopInitiator, PromptExecutionOutcome,
    PromptExecutionReport, PromptTrace, PromptVersionReference,
    ProviderCredentialProbeAuthentication, ProviderCredentialProbeProtocol,
    ProviderCredentialProbeRequest, ProviderCredentialValidationResult,
    ProviderCredentialValidationStatus, ReadinessView, RegisterApiAgentInput, ReportedUsageTotals,
    ResizeAgentTerminalRequest, SaveOnePieceProviderConfigInput, SaveOnePieceProviderProfileInput,
    SeatTurnOwnership, SeatTurnTerminal, SendMessageRequest, StartedAgentMessage,
    StartedGenerationProcess, StopAgentTerminalRequest, StopGenerationResult,
    StoredOnePieceProviderConfig, StoredOnePieceProviderProfile, ToolApprovalDecision,
    ToolDefinition, ToolLifecycleEvent, ToolLifecyclePhase, ToolUseBlock, UpdateApiAgentInput,
    ValidateOnePieceProviderCredentialInput, WorkflowLaunchOutcome, WorkflowLaunchRequest,
    WorkflowView, INTERFACE_FORMAT_ANTHROPIC, INTERFACE_FORMAT_OPENAI_COMPATIBLE,
};

#[cfg(test)]
pub(crate) use models::GenerationProcessFailureKind;
pub(crate) use models::SeatTurnStatus;
pub(crate) use ports::SeatTurnCompletionPort;
pub(crate) use ports::{
    AgentAvailabilityGateway, AgentCliProfileGateway, AgentClockPort, AgentCodeRetrievalHit,
    AgentCodeRetrievalOutcome, AgentCodeRetrievalPort, AgentCoreInstructionsPort, AgentEventPort,
    AgentGenerationPort, AgentLoggingPort, AgentMcpToolPort, AgentMemoryExtractionPort,
    AgentMemoryPort, AgentMessageTerminalCompletionPort, AgentPermissionPort,
    AgentPersonalizationPort, AgentProcessEventSink, AgentProcessGateway, AgentRegistryRepository,
    AgentRetrievalHit, AgentRetrievalOutcome, AgentRetrievalPort, AgentSessionGateway,
    AgentSkillPort, AgentTaskPort, AgentTerminalEventPort, AgentTerminalGateway,
    AgentWorkflowRepository, ApiAgentGateway, ApiCredentialPort, ConversationHistoryPort,
    EffectivePromptGateway, LoopExecutionControlPort, LoopExecutionLeasePort,
    LoopGenerationControlPort, LoopGitStatePort, LoopIterationRepository, LoopLoggingPort,
    LoopProjectPort, LoopRepository, LoopRoleGenerationCompletionPort, LoopRoleSessionPort,
    LoopSessionRecoveryPort, LoopVerificationProcessPort, LoopVerifierContextPort,
    LoopVerifierGenerationPort, LoopWorkerGenerationPort, OnePieceModelDiscoveryPort,
    OnePiecePlanningPort, ToolApprovalPort,
};
pub(crate) use provider::{
    AgentProvider, AgentProviderError, ProviderGenerationInvocationRequest,
    ProviderInteractiveInvocationRequest, ProviderInteractiveInvocationSpec,
    ProviderInvocationSpec, ProviderOutputFormat, ProviderPromptDelivery, ProviderRegistry,
};
pub(crate) use seat_turn::{SeatTurnAssignment, SeatTurnStop};
pub(crate) use service::{AgentRuntimeApplicationPorts, AgentRuntimeApplicationService};
pub(crate) use terminal_service::{AgentTerminalApplicationPorts, AgentTerminalApplicationService};
pub(crate) use tool_catalog::{
    plan_mode_tool_catalog, recall_tool_definition, search_code_tool_definition, tool_catalog,
    EDIT_TOOL_NAME, FILE_TOOL_NAME, GLOB_TOOL_NAME, GREP_TOOL_NAME, MCP_TOOL_NAME_PREFIX,
    RECALL_TOOL_NAME, REMEMBER_TOOL_NAME, SEARCH_CODE_TOOL_NAME, SHELL_TOOL_NAME,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod terminal_service_tests;

#[cfg(test)]
mod loop_service_tests;

#[cfg(test)]
mod loop_control_tests;

#[cfg(test)]
mod loop_progress_tests;

#[cfg(test)]
mod loop_orchestrator_tests;

#[cfg(test)]
mod loop_recovery_tests;

#[cfg(test)]
mod loop_verification_tests;

#[cfg(test)]
mod loop_verifier_tests;

#[cfg(test)]
mod loop_worker_tests;
