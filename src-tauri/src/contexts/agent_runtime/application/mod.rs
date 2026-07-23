mod error;
mod models;
mod ports;
mod service;
mod terminal_service;

pub(crate) use error::AgentRuntimeApplicationError;
#[cfg(test)]
pub(crate) use models::AgentLaunchView;
pub(crate) use models::{
    AgentChatConfiguration, AgentEvent, AgentFileReference, AgentLog, AgentLogLevel, AgentMessage,
    AgentMessageSource, AgentOperation, AgentSession, AgentSessionDetails, AgentTerminalCapability,
    AgentTerminalEvent, AgentTerminalInputRequest, AgentTerminalProcessRequest,
    AgentTerminalSession, AgentTerminalSize, AgentTerminalState, AgentUsageRecord, AgentView,
    CliProfileSnapshot, CompleteAgentMessage, EffectivePrompt, GenerationCancellation,
    GenerationLease, GenerationProcessEvent, GenerationProcessRequest, LaunchWorkflowResult,
    MessageTokenUsage, NewAgentMessage, OpenAgentTerminalRequest, ProcessStopInitiator,
    PromptTrace, ReadinessView, ResizeAgentTerminalRequest, SendMessageRequest,
    StartedGenerationProcess, StopAgentTerminalRequest, StopGenerationResult, ToolLifecycleEvent,
    ToolLifecyclePhase, ToolUseBlock, WorkflowLaunchOutcome, WorkflowLaunchRequest, WorkflowView,
};
pub(crate) use ports::{
    AgentAvailabilityGateway, AgentCliProfileGateway, AgentClockPort, AgentEventPort,
    AgentGenerationPort, AgentLoggingPort, AgentProcessEventSink, AgentProcessGateway,
    AgentRegistryRepository, AgentSessionGateway, AgentTaskPort, AgentTerminalEventPort,
    AgentTerminalGateway, AgentWorkflowRepository, EffectivePromptGateway,
};
pub(crate) use service::{AgentRuntimeApplicationPorts, AgentRuntimeApplicationService};
pub(crate) use terminal_service::{AgentTerminalApplicationPorts, AgentTerminalApplicationService};

#[cfg(test)]
mod tests;
