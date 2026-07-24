mod coordination;
mod error;
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
mod models;
mod ports;
mod service;
mod terminal_service;

pub(crate) use crate::contexts::agent_runtime::domain::LoopVerifierRecommendation;
pub(crate) use coordination::{
    CoordinationApplicationPorts, CoordinationApplicationService, CoordinationExecutionOutput,
    CoordinationExecutionRequest, CoordinationExecutionResult, CoordinationIdPort,
    CoordinationNodeExecutor, CoordinationOperationPort, CoordinationRepository,
    StartCoordinationRequest, StartCoordinationResultView,
};
pub(crate) use error::AgentRuntimeApplicationError;
pub(crate) use loop_control::{LoopControlApplicationPorts, LoopControlApplicationService};
pub(crate) use loop_models::{
    ContinueLoopRequest, LoopDefinitionView, LoopEvidenceView, LoopGitStateEntryView,
    LoopGitStateView, LoopIterationView, LoopRoleSessionRequest, LoopRunView,
    LoopVerificationBatchResult, LoopVerifierResult, PreparedLoopWorktree,
    RunLoopVerificationRequest, SaveLoopDefinitionRequest, SaveLoopVerifierResultRequest,
    StartLoopResultView, StartLoopVerifierRequest, StartLoopWorkerRequest, StartedLoopVerifierView,
    StartedLoopWorkerView,
};
#[cfg(test)]
pub(crate) use loop_models::{LoopLimitsView, LoopVerificationCommandView};
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
    AgentChatConfiguration, AgentEvent, AgentFileReference, AgentLog, AgentLogLevel, AgentMessage,
    AgentMessageSource, AgentOperation, AgentSession, AgentSessionDetails, AgentTerminalCapability,
    AgentTerminalEvent, AgentTerminalInputRequest, AgentTerminalProcessRequest,
    AgentTerminalSession, AgentTerminalSize, AgentTerminalState, AgentUsageRecord, AgentView,
    CliProfileSnapshot, CompleteAgentMessage, EffectivePrompt, GenerationCancellation,
    GenerationLease, GenerationProcessEvent, GenerationProcessFailure,
    GenerationProcessFailureKind, GenerationProcessRequest, LaunchWorkflowResult, LoopLog,
    LoopOperationContext, LoopOperationKind, LoopRoleGenerationOutcome,
    LoopRoleGenerationOwnership, LoopRoleGenerationTerminal, LoopVerificationCancellation,
    LoopVerificationProcessRequest, LoopVerificationProcessResult, LoopVerificationProcessStatus,
    MessageTokenUsage, NewAgentMessage, OpenAgentTerminalRequest, PendingPromptExecution,
    ProcessStopInitiator, PromptExecutionOutcome, PromptExecutionReport, PromptTrace,
    PromptVersionReference, ReadinessView, ResizeAgentTerminalRequest, SendMessageRequest,
    StartedGenerationProcess, StopAgentTerminalRequest, StopGenerationResult, ToolLifecycleEvent,
    ToolLifecyclePhase, ToolUseBlock, WorkflowLaunchOutcome, WorkflowLaunchRequest, WorkflowView,
};
pub(crate) use ports::{
    AgentAvailabilityGateway, AgentCliProfileGateway, AgentClockPort, AgentEventPort,
    AgentGenerationPort, AgentLoggingPort, AgentProcessEventSink, AgentProcessGateway,
    AgentRegistryRepository, AgentSessionGateway, AgentTaskPort, AgentTerminalEventPort,
    AgentTerminalGateway, AgentWorkflowRepository, EffectivePromptGateway,
    LoopExecutionControlPort, LoopExecutionLeasePort, LoopGenerationControlPort, LoopGitStatePort,
    LoopIterationRepository, LoopLoggingPort, LoopProjectPort, LoopRepository,
    LoopRoleGenerationCompletionPort, LoopRoleSessionPort, LoopVerificationProcessPort,
    LoopVerifierContextPort, LoopVerifierGenerationPort, LoopWorkerGenerationPort,
};
pub(crate) use service::{AgentRuntimeApplicationPorts, AgentRuntimeApplicationService};
pub(crate) use terminal_service::{AgentTerminalApplicationPorts, AgentTerminalApplicationService};

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
