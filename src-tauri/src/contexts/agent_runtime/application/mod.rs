mod error;
mod models;
mod ports;
mod service;

pub(crate) use error::AgentRuntimeApplicationError;
#[cfg(test)]
pub(crate) use models::AgentLaunchView;
pub(crate) use models::{
    AgentChatConfiguration, AgentEvent, AgentFileReference, AgentLog, AgentLogLevel, AgentMessage,
    AgentOperation, AgentSession, AgentSessionDetails, AgentUsageRecord, AgentView,
    CliProfileSnapshot, CompleteAgentMessage, EffectivePrompt, GenerationCancellation,
    GenerationLease, GenerationProcessEvent, GenerationProcessRequest, LaunchWorkflowResult,
    MessageTokenUsage, NewAgentMessage, PromptTrace, ReadinessView, SendMessageRequest,
    StartedGenerationProcess, StopGenerationResult, ToolUseBlock, WorkflowLaunchOutcome,
    WorkflowLaunchRequest, WorkflowView,
};
pub(crate) use ports::{
    AgentAvailabilityGateway, AgentCliProfileGateway, AgentClockPort, AgentEventPort,
    AgentGenerationPort, AgentLoggingPort, AgentProcessEventSink, AgentProcessGateway,
    AgentRegistryRepository, AgentSessionGateway, AgentTaskPort, AgentWorkflowRepository,
    EffectivePromptGateway,
};
pub(crate) use service::{AgentRuntimeApplicationPorts, AgentRuntimeApplicationService};

#[cfg(test)]
mod tests;
