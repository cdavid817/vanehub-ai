mod context_analysis;
#[cfg(test)]
mod context_analysis_tests;
mod context_quality_query;
mod context_quality_recording;
mod context_reinjection;
#[cfg(test)]
mod context_reinjection_tests;
mod error;
mod execution_policy;
mod existing_tool_registry;
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
mod native_tools;
mod onepiece_provider_catalog;
mod ports;
#[cfg(test)]
mod ports_contract_tests;
mod provider;
mod seat_turn;
#[cfg(test)]
mod seat_turn_tests;
mod service;
mod terminal_service;
mod tool_catalog;
mod utility_delegation;

pub(crate) use crate::contexts::agent_runtime::domain::LoopVerifierRecommendation;
pub(crate) use context_analysis::{ContextAnalysisInput, ContextAnalysisService};
pub(crate) use context_quality_query::ContextQualityQueryService;
pub(crate) use context_quality_recording::ContextQualityRecorder;
#[allow(unused_imports)]
pub(crate) use context_reinjection::{
    AuthoritativeContextValue, ContextReinjectionBudget, ContextReinjectionEvidence,
    ContextReinjectionFailure, ContextReinjectionKind, ContextReinjectionResult,
    ContextReinjectionService, ReinjectedContextValue,
};
pub(crate) use error::AgentRuntimeApplicationError;
pub(crate) use execution_policy::{resolve_effective_execution_policy, SessionExecutionMode};
pub(crate) use existing_tool_registry::{ExistingToolHandler, ExistingToolHandlerRegistry};
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
    LoopVerificationEvidenceFact, LoopVerificationEvidencePort,
};
pub(crate) use loop_verifier::{LoopVerifierApplicationPorts, LoopVerifierApplicationService};
pub(crate) use loop_worker::{LoopWorkerApplicationPorts, LoopWorkerApplicationService};
#[cfg(test)]
pub(crate) use models::AgentLaunchView;
pub(crate) use models::{
    format_memory_section, ActiveGenerationCorrelation, AgentChatConfiguration,
    AgentCoreInstructions, AgentEvent, AgentFileReference, AgentInvocationUsage, AgentLog,
    AgentLogLevel, AgentMemory, AgentMessage, AgentMessageSource, AgentMessageTerminal,
    AgentMessageTerminalOutcome, AgentMessageTerminalReceiver, AgentOperation, AgentSession,
    AgentSessionDetails, AgentSessionSeat, AgentSkillReadRequest, AgentTerminalCapability,
    AgentTerminalEvent, AgentTerminalInputRequest, AgentTerminalProcessRequest,
    AgentTerminalSession, AgentTerminalSize, AgentTerminalState, AgentToolCallOutcome,
    AgentUsageAccountingKind, AgentUsageOverlap, AgentUsageRecord, AgentView, ApiProviderConfig,
    BoundSkillPrompt, CliProfileSnapshot, CompleteAgentMessage,
    DiscoverOnePieceProviderModelsInput, DurableAgentGenerationMessages,
    DurableAgentGenerationStart, EffectivePrompt, EmbeddingEndpointView, ExecutionToolMode,
    GenerationCancellation, GenerationLease, GenerationProcessEvent, GenerationProcessFailure,
    GenerationProcessRequest, LaunchWorkflowResult, LoopLog, LoopOperationContext,
    LoopOperationKind, LoopRoleGenerationOutcome, LoopRoleGenerationOwnership,
    LoopRoleGenerationTerminal, LoopVerificationCancellation, LoopVerificationProcessRequest,
    LoopVerificationProcessResult, LoopVerificationProcessStatus, MemorySource, MessageTokenUsage,
    NewAgentMessage, OnePieceDiscoveredModel, OnePieceModelDiscoveryRequest,
    OnePieceProviderConfig, OnePieceProviderEndpoint, OnePieceProviderModelDiscoveryResult,
    OnePieceProviderModelOption, OnePieceProviderPreset, OnePieceProviderProfile,
    OnePieceProviderProfiles, OpenAgentTerminalRequest, OrchestrationCorrelation,
    OrchestrationExecutionProfile, PendingPromptExecution, PersonalizationSettings,
    ProcessStopInitiator, PromptExecutionOutcome, PromptExecutionReport, PromptTrace,
    PromptVersionReference, ProviderCredentialProbeAuthentication, ProviderCredentialProbeProtocol,
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
pub(crate) use ports::ContextQualityRepository;

#[cfg(test)]
pub(crate) use models::{OnePiecePlanningRequest, OnePiecePlanningResult};

#[cfg(test)]
pub(crate) use models::GenerationProcessFailureKind;
pub(crate) use models::SeatTurnStatus;
#[allow(unused_imports)]
pub(crate) use native_tools::{
    is_onepiece_only, web_native_tool_handlers, ApplyDelegationChangesNativeToolHandler,
    ArtifactNativeToolHandler, ArtifactPort, ArtifactRecord, BrowserAutomationPort,
    BrowserHandoffControlPort, BrowserNativeToolHandler, CanonicalToolResource, ChangeSetApplyPort,
    ChangeSetApplyRecord, ChangeSetFileRecord, ChangeSetRecord, ChangeSetStatus, CliDelegationPort,
    CodeExecutionNativeToolHandler, CodeExecutionPort, DelegateCliNativeToolHandler,
    DelegationAttemptRecord, DelegationMode, DelegationRecord, DelegationStatus, DelegationTarget,
    FileChangeKind, ManualApplyDelegationRequest, ManualNativeToolAuthorityPort,
    ManualNativeToolOperationPort, ManualNativeToolRequest, ManualNativeToolResult,
    ManualNativeToolService, ManualStartDelegationRequest, NativeToolApprovalWitness,
    NativeToolAuthorizationStatus, NativeToolDefinition, NativeToolDispatchError,
    NativeToolDispatchRequest, NativeToolDispatcher, NativeToolErrorCode,
    NativeToolExecutionContext, NativeToolExecutionMode, NativeToolHandler, NativeToolHandlerError,
    NativeToolLimitProfile, NativeToolLogEvent, NativeToolLogEventKind, NativeToolLogIdentity,
    NativeToolOperation, NativeToolPermissionRequest, NativeToolPortRequest,
    NativeToolPrivateLogData, NativeToolProgress, NativeToolProgressPhase, NativeToolProgressSink,
    NativeToolReadinessReasonCode, NativeToolRegistry, NativeToolRegistryError,
    NativeToolResultEnvelope, NativeToolResultStatus, NativeToolSafeLogMetadata, OcrInferencePort,
    OcrNativeToolHandler, OnePieceToolFeatureGates, PreparedNativeToolDispatch, RecoveryRecord,
    RecoveryStatus, StoredToolOperation, StoredToolOperationStatus, ToolEligibility,
    ToolEligibilityContext, ToolResourceKind, ValidatedNativeToolInput, WebResearchPort,
    NATIVE_TOOL_CONTRACT_VERSION, ONEPIECE_AGENT_ID, ONEPIECE_ONLY_TOOL_NAMES,
};
#[cfg(test)]
pub(crate) use ports::OnePiecePlanningPort;
pub(crate) use ports::SeatTurnCompletionPort;
pub(crate) use ports::{
    AgentAvailabilityGateway, AgentCliProfileGateway, AgentClockPort, AgentCodeRetrievalHit,
    AgentCodeRetrievalOutcome, AgentCodeRetrievalPort, AgentCoreInstructionsPort, AgentEventPort,
    AgentGenerationPort, AgentLoggingPort, AgentMcpToolPort, AgentMemoryExtractionPort,
    AgentMemoryPort, AgentMessageTerminalCompletionPort, AgentPermissionPort,
    AgentPersonalizationPort, AgentProcessEventSink, AgentProcessGateway, AgentRegistryRepository,
    AgentRetrievalHit, AgentRetrievalOutcome, AgentRetrievalPort, AgentSessionGateway,
    AgentSkillPort, AgentTaskPort, AgentTerminalEventPort, AgentTerminalGateway,
    AgentWorkflowRepository, ApiAgentGateway, ApiCredentialPort, AuthoritativeContextPort,
    ConversationHistoryPort, EffectivePromptGateway, LoopExecutionControlPort,
    LoopExecutionLeasePort, LoopGenerationControlPort, LoopGitStatePort, LoopIterationRepository,
    LoopLoggingPort, LoopProjectPort, LoopRepository, LoopRoleGenerationCompletionPort,
    LoopRoleSessionPort, LoopSessionRecoveryPort, LoopVerificationProcessPort,
    LoopVerifierContextPort, LoopVerifierGenerationPort, LoopWorkerGenerationPort,
    OnePieceModelDiscoveryPort, ToolApprovalPort,
};
#[allow(unused_imports)]
pub(crate) use ports::{
    AgentCodeDiagnostic, AgentCodeHover, AgentCodeIntelligenceContext,
    AgentCodeIntelligenceMetadata, AgentCodeIntelligenceOutcome, AgentCodeIntelligencePending,
    AgentCodeIntelligencePort, AgentCodeIntelligenceResponderPort, AgentCodeIntelligenceStatus,
    AgentCodeLocation, AgentCodeRange, AgentDocumentInput, AgentDocumentPositionInput,
    AgentWorkspaceMutation, AgentWorkspaceMutationPort,
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
    code_intelligence_tool_definitions, delegate_utility_skill_tool_definition,
    plan_mode_tool_catalog, recall_tool_definition, search_code_tool_definition, tool_catalog,
    DELEGATE_UTILITY_SKILL_TOOL_NAME, EDIT_TOOL_NAME, FILE_TOOL_NAME, FIND_DEFINITION_TOOL_NAME,
    FIND_REFERENCES_TOOL_NAME, GET_DIAGNOSTICS_TOOL_NAME, GET_HOVER_TOOL_NAME, GLOB_TOOL_NAME,
    GREP_TOOL_NAME, LIST_SKILLS_TOOL_NAME, LOAD_SKILL_TOOL_NAME, MCP_TOOL_NAME_PREFIX,
    READ_SKILL_RESOURCE_TOOL_NAME, RECALL_TOOL_NAME, REMEMBER_TOOL_NAME, SEARCH_CODE_TOOL_NAME,
    SHELL_KILL_TOOL_NAME, SHELL_OUTPUT_TOOL_NAME, SHELL_TOOL_NAME,
};
pub(crate) use utility_delegation::{
    UtilityChildExecutionOutcome, UtilityChildExecutionPort, UtilityDelegationApplicationPorts,
    UtilityDelegationApplicationService, UtilityDelegationEvidenceFact,
    UtilityDelegationEvidencePort, UtilityDelegationLifecycleFact, UtilityDelegationLifecyclePort,
    UtilityDelegationResolutionPort,
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
