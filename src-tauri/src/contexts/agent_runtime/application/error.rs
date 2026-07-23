use crate::contexts::agent_runtime::domain::AgentRuntimeDomainError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentRuntimeApplicationError {
    Domain(AgentRuntimeDomainError),
    Validation(String),
    AgentNotFound(String),
    SessionNotFound(String),
    MessageNotFound(String),
    NoActiveAgent,
    AgentUnavailable(String),
    UnsupportedInteractionMode(String),
    GenerationConflict(String),
    PolicyDenied { session_id: String, action: String },
    Registry(String),
    Workflow(String),
    Session(String),
    CliProfile(String),
    Prompt(String),
    Process(String),
    Operation(String),
    Loop(String),
    Coordination(String),
    VerificationPolicy(String),
    VerificationProcess(String),
    Logging(String),
    Event(String),
    Generation(String),
}

impl fmt::Display for AgentRuntimeApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(formatter),
            Self::Validation(message) => formatter.write_str(message),
            Self::AgentNotFound(agent_id) => write!(formatter, "Agent not found: {agent_id}"),
            Self::SessionNotFound(session_id) => {
                write!(formatter, "Session not found: {session_id}")
            }
            Self::MessageNotFound(message_id) => {
                write!(formatter, "Message not found: {message_id}")
            }
            Self::NoActiveAgent => formatter.write_str("No active agent selected."),
            Self::AgentUnavailable(message) => write!(formatter, "Agent unavailable: {message}"),
            Self::UnsupportedInteractionMode(mode) => {
                write!(formatter, "Unsupported interaction mode: {mode}")
            }
            Self::GenerationConflict(session_id) => write!(
                formatter,
                "A generation is already active for session {session_id}."
            ),
            Self::PolicyDenied { session_id, action } => write!(
                formatter,
                "Verifier session {session_id} cannot perform Agent action: {action}"
            ),
            Self::Registry(message) => write!(formatter, "agent registry error: {message}"),
            Self::Workflow(message) => write!(formatter, "agent workflow error: {message}"),
            Self::Session(message) => write!(formatter, "agent session error: {message}"),
            Self::CliProfile(message) => write!(formatter, "CLI profile error: {message}"),
            Self::Prompt(message) => write!(formatter, "effective prompt error: {message}"),
            Self::Process(message) => write!(formatter, "agent process error: {message}"),
            Self::Operation(message) => write!(formatter, "agent operation error: {message}"),
            Self::Loop(message) => write!(formatter, "Loop runtime error: {message}"),
            Self::Coordination(message) => {
                write!(formatter, "Multi-Agent coordination error: {message}")
            }
            Self::VerificationPolicy(message) => {
                write!(
                    formatter,
                    "verification policy rejected execution: {message}"
                )
            }
            Self::VerificationProcess(message) => {
                write!(formatter, "verification process failed: {message}")
            }
            Self::Logging(message) => write!(formatter, "agent logging error: {message}"),
            Self::Event(message) => write!(formatter, "agent event error: {message}"),
            Self::Generation(message) => write!(formatter, "agent generation error: {message}"),
        }
    }
}

impl std::error::Error for AgentRuntimeApplicationError {}

impl From<AgentRuntimeDomainError> for AgentRuntimeApplicationError {
    fn from(error: AgentRuntimeDomainError) -> Self {
        match error {
            AgentRuntimeDomainError::AgentUnavailable(message) => Self::AgentUnavailable(message),
            AgentRuntimeDomainError::InteractionModeNotSupported { mode, .. } => {
                Self::UnsupportedInteractionMode(mode)
            }
            AgentRuntimeDomainError::NoActiveAgent => Self::NoActiveAgent,
            other => Self::Domain(other),
        }
    }
}
