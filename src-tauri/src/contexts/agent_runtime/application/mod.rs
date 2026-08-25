mod context_analysis;
#[cfg(test)]
mod context_analysis_tests;
mod context_engine;
mod context_quality_query;
mod context_quality_recording;
mod context_reinjection;
#[cfg(test)]
mod context_reinjection_tests;
mod error;
mod execution_policy;
mod existing_tool_registry;
mod expert_role;
mod local_model_discovery;
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
mod runner;
#[cfg(test)]
mod runner_tests;
mod seat_turn;
#[cfg(test)]
mod seat_turn_tests;
mod service;
mod terminal_service;
mod tool_catalog;
mod utility_delegation;

pub(crate) use crate::contexts::agent_runtime::domain::LoopVerifierRecommendation;
pub(crate) use context_analysis::{ContextAnalysisInput, ContextAnalysisService};
#[allow(unused_imports)]
pub(crate) use context_engine::{
    ContextCandidateSource, ContextEngineClockPort, ContextEngineDiagnostic,
    ContextEngineDiagnosticPort, ContextEngineOutcome, ContextEngineService,
    ContextManifestQueryService, ContextManifestRepository, ContextPlan, ContextSourceResult,
    ProjectedContextEvidence,
};
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
pub(crate) use local_model_discovery::LocalModelDiscoveryService;
pub(crate) use loop_control::{LoopControlApplicationPorts, LoopControlApplicationService};
#[cfg(test)]
pub(crate) use loop_models::LoopLimitsView;
pub(crate) use loop_models::LoopVerificationCommandView;
pub(crate) use loop_models::{
    ContinueLoopRequest, LoopChildRecoveryDecision, LoopChildRecoveryProjection,
    LoopDefinitionView, LoopEvidenceView, LoopGitStateEntryView, LoopGitStateView,
    LoopIterationView, LoopOwnedRecoverySession, LoopReadinessCheckView, LoopReadinessReportView,
    LoopRoleSessionRequest, LoopRunView, LoopVerificationBatchResult, LoopVerifierResult,
    PreparedLoopWorktree, RunLoopVerificationRequest, SaveLoopDefinitionRequest,
    SaveLoopVerifierResultRequest, StartLoopResultView, StartLoopVerifierRequest,
    StartLoopWorkerRequest, StartedLoopVerifierView, StartedLoopWorkerView,
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
// `format_memory_bodies` and `MemoryIndexBounds` are consumed by tasks 2 and 3, which add the
// relevance selection that produces bodies. Until then only the index half of the split is wired.
#[allow(unused_imports)]
pub(crate) use models::{
    format_memory_bodies, format_memory_index, resume_thread_for, ActiveGenerationCorrelation,
    AgentChatConfiguration, AgentCoreInstructions, AgentEvent, AgentFileReference,
    AgentInvocationUsage, AgentLog, AgentLogLevel, AgentMemory, AgentMemoryAccess,
    AgentMemoryDelivery, AgentMemoryRef, AgentMessage, AgentMessageSource, AgentMessageTerminal,
    AgentMessageTerminalOutcome, AgentMessageTerminalReceiver, AgentOperation,
    AgentPersonalizationSnapshot, AgentSession, AgentSessionDetails, AgentSessionRunnerTarget,
    AgentSessionSeat, AgentSkillReadRequest, AgentTerminalCapability, AgentTerminalEvent,
    AgentTerminalInputRequest, AgentTerminalProcessRequest, AgentTerminalSession,
    AgentTerminalSize, AgentTerminalState, AgentToolCallOutcome, AgentUsageAccountingKind,
    AgentUsageOverlap, AgentUsageRecord, AgentView, ApiProviderConfig, BoundSkillPrompt,
    CliProfileSnapshot, CompleteAgentMessage, DiscoverOnePieceProviderModelsInput,
    DurableAgentGenerationMessages, DurableAgentGenerationStart, EffectivePrompt,
    EmbeddingEndpointView, FrozenEndpointProfile, GenerationCancellation, GenerationLease,
    GenerationProcessEvent, GenerationProcessFailure, GenerationProcessRequest, HybridRoutePreview,
    HybridRoutePreviewInput, LaunchWorkflowResult, LocalEndpointVerificationRequest,
    LocalModelDiscoveryResult, LocalModelEndpointCandidate, LoopLog, LoopOperationContext,
    LoopOperationKind, LoopRoleGenerationOutcome, LoopRoleGenerationOwnership,
    LoopRoleGenerationTerminal, LoopVerificationCancellation, LoopVerificationProcessRequest,
    LoopVerificationProcessResult, LoopVerificationProcessStatus, MemoryIndexBounds, MemorySource,
    MessageTokenUsage, NewAgentMessage, OnePieceDiscoveredModel, OnePieceModelDiscoveryRequest,
    OnePieceProviderConfig, OnePieceProviderEndpoint, OnePieceProviderModelDiscoveryResult,
    OnePieceProviderModelOption, OnePieceProviderPreset, OnePieceProviderProfile,
    OnePieceProviderProfiles, OpenAgentTerminalRequest, PendingPromptExecution,
    PersonalizationSettings, ProcessStopInitiator, PromptExecutionOutcome, PromptExecutionReport,
    PromptTrace, PromptVersionReference, ProviderCredentialProbeAuthentication,
    ProviderCredentialProbeProtocol, ProviderCredentialProbeRequest,
    ProviderCredentialValidationResult, ProviderCredentialValidationStatus, ReadinessView,
    RecoverSessionResult, RegisterApiAgentInput, ReportedUsageTotals, ResizeAgentTerminalRequest,
    SaveCustomOnePieceProviderProfileInput, SaveMemoryInput, SaveOnePieceProviderConfigInput,
    SaveOnePieceProviderProfileInput, SeatTurnOwnership, SeatTurnTerminal, SendMessageRequest,
    SkillToolUseProvenance, StartedAgentMessage, StartedGenerationProcess,
    StopAgentTerminalRequest, StopGenerationResult, StoredEndpointProfileMetadata,
    StoredHybridRoutingRule, StoredOnePieceProviderConfig, StoredOnePieceProviderProfile,
    ToolApprovalDecision, ToolDefinition, ToolLifecycleEvent, ToolLifecyclePhase, ToolUseBlock,
    UpdateApiAgentInput, ValidateOnePieceProviderCredentialInput, WorkflowLaunchOutcome,
    WorkflowLaunchRequest, WorkflowView, CLI_MEMORY_INDEX_BOUNDS, INTERFACE_FORMAT_ANTHROPIC,
    INTERFACE_FORMAT_OPENAI_COMPATIBLE, ONEPIECE_MEMORY_INDEX_BOUNDS,
};
pub(crate) use ports::ContextQualityRepository;

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
    NativeToolOperation, NativeToolPermissionRequest, NativeToolPersistencePort,
    NativeToolPortRequest, NativeToolPrivateLogData, NativeToolProgress, NativeToolProgressPhase,
    NativeToolProgressSink, NativeToolReadinessReasonCode, NativeToolRegistry,
    NativeToolRegistryError, NativeToolResultEnvelope, NativeToolResultStatus,
    NativeToolSafeLogMetadata, OcrInferencePort, OcrNativeToolHandler, OnePieceToolFeatureGates,
    PreparedNativeToolDispatch, RecoveryRecord, RecoveryStatus, StoredToolOperation,
    StoredToolOperationStatus, SubagentNativeToolHandler, SubagentPort, ToolEligibility,
    ToolEligibilityContext, ToolResourceKind, ValidatedNativeToolInput, WebResearchPort,
    IMAGE_ARTIFACT_METADATA_KEY, MAX_SUBAGENT_DURATION_MS, NATIVE_TOOL_CONTRACT_VERSION,
    ONEPIECE_AGENT_ID, ONEPIECE_ONLY_TOOL_NAMES,
};
pub(crate) use ports::SeatTurnCompletionPort;
pub(crate) use ports::{
    AgentAuthenticationPolicyPort, AgentAvailabilityGateway, AgentCandidateOutcome,
    AgentCandidateSubmission, AgentCliProfileGateway, AgentClockPort, AgentCodeRetrievalHit,
    AgentCodeRetrievalOutcome, AgentCodeRetrievalPort, AgentCoreInstructionsPort, AgentEventPort,
    AgentGenerationPort, AgentLoggingPort, AgentMcpToolPort, AgentMemoryBody,
    AgentMemoryExtractionPort, AgentMemoryPort, AgentMemoryProposal, AgentMemorySelectionPort,
    AgentMessageTerminalCompletionPort, AgentPermissionPort, AgentPersonalizationPort,
    AgentPersonalizationSnapshotPort, AgentProcessEventSink, AgentProcessGateway,
    AgentProposalOrigin, AgentRegistryRepository, AgentRetrievalHit, AgentRetrievalOutcome,
    AgentRetrievalPort, AgentSessionGateway, AgentSkillPort, AgentTaskPort, AgentTerminalEventPort,
    AgentTerminalGateway, AgentWorkflowRepository, ApiAgentGateway, ApiCredentialPort,
    AuthoritativeContextPort, CanonicalLoopSignal, CanonicalRunLinks, CanonicalRunOutcome,
    CanonicalRunSignal, ConversationHistoryPort, EffectivePromptGateway,
    GenerationPersonalizationContext, LocalModelDiscoveryPort, LoopExecutionControlPort,
    LoopExecutionLeasePort, LoopGenerationControlPort, LoopGitStatePort, LoopIterationRepository,
    LoopLoggingPort, LoopProjectPort, LoopRepository, LoopRoleGenerationCompletionPort,
    LoopRoleSessionPort, LoopSessionRecoveryPort, LoopVerificationProcessPort,
    LoopVerifierContextPort, LoopVerifierGenerationPort, LoopWorkerGenerationPort,
    OnePieceModelDiscoveryPort, RunnerDiscoveryPort, ToolApprovalPort,
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
    ProviderInvocationSpec, ProviderOptionRequest, ProviderOutputFormat, ProviderPermissionMode,
    ProviderPromptDelivery, ProviderRegistry,
};
#[allow(unused_imports)]
pub(crate) use runner::{
    AgentRunner, PreparedRunnerLaunch, RunnerCapabilities, RunnerDescriptor, RunnerError,
    RunnerErrorKind, RunnerEvent, RunnerHandle, RunnerInspection, RunnerKind, RunnerLaunchSpec,
    RunnerPermissionContext, RunnerPermissionPort, RunnerPolicyWitness, RunnerRecoveryMode,
    RunnerReference, RunnerSelection,
};
pub(crate) use seat_turn::{SeatTurnAssignment, SeatTurnStop};
pub(crate) use service::{AgentRuntimeApplicationPorts, AgentRuntimeApplicationService};
pub(crate) use terminal_service::{AgentTerminalApplicationPorts, AgentTerminalApplicationService};
pub(crate) use tool_catalog::{
    ask_user_question_tool_definition, code_intelligence_tool_definitions,
    delegate_utility_skill_tool_definition, plan_mode_tool_catalog, recall_tool_definition,
    search_code_tool_definition, tool_catalog, ASK_USER_QUESTION_TOOL_NAME,
    DELEGATE_UTILITY_SKILL_TOOL_NAME, EDIT_TOOL_NAME, EXIT_PLAN_MODE_TOOL_NAME, FILE_TOOL_NAME,
    FIND_DEFINITION_TOOL_NAME, FIND_REFERENCES_TOOL_NAME, GET_DIAGNOSTICS_TOOL_NAME,
    GET_HOVER_TOOL_NAME, GLOB_TOOL_NAME, GREP_TOOL_NAME, LIST_SKILLS_TOOL_NAME,
    LOAD_SKILL_TOOL_NAME, MAX_PLAN_CHARS, MAX_QUESTION_CHARS, MAX_QUESTION_OPTIONS,
    MAX_QUESTION_OPTION_CHARS, MCP_TOOL_NAME_PREFIX, MIN_QUESTION_OPTIONS, NOTEBOOK_TOOL_NAME,
    READ_SKILL_RESOURCE_TOOL_NAME, RECALL_TOOL_NAME, REMEMBER_TOOL_NAME, SEARCH_CODE_TOOL_NAME,
    SHELL_KILL_TOOL_NAME, SHELL_OUTPUT_TOOL_NAME, SHELL_TOOL_NAME, TODO_WRITE_TOOL_NAME,
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
