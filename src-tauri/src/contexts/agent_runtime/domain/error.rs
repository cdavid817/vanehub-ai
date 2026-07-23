use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum AgentRuntimeDomainError {
    #[error("{0} cannot be empty.")]
    RequiredValue(&'static str),
    #[error("{0} contains invalid control characters.")]
    ControlCharacters(&'static str),
    #[error("unsupported interaction mode: {0}")]
    UnsupportedInteractionMode(String),
    #[error("Agent is unavailable: {0}")]
    AgentUnavailable(String),
    #[error("Agent '{agent_id}' does not support interaction mode '{mode}'.")]
    InteractionModeNotSupported { agent_id: String, mode: String },
    #[error("Workflow selection must contain both an agent and an interaction mode.")]
    IncompleteWorkflowSelection,
    #[error("No active agent selected.")]
    NoActiveAgent,
    #[error("Cannot transition agent lifecycle from '{from}' to '{to}'.")]
    InvalidLifecycleTransition { from: String, to: String },
    #[error("Generation message id cannot be empty.")]
    GenerationMessageRequired,
    #[error("Cannot transition generation from '{from}' to '{to}'.")]
    InvalidGenerationTransition { from: String, to: String },
    #[error("invalid Loop {0}.")]
    InvalidLoopValue(&'static str),
    #[error("invalid Loop limit: {0}.")]
    InvalidLoopLimit(&'static str),
    #[error("Cannot transition Loop from '{from}' to '{to}'.")]
    InvalidLoopTransition { from: String, to: String },
    #[error("Loop limit reached: {0}.")]
    LoopLimitReached(&'static str),
    #[error("invalid Multi-Agent coordination: {0}.")]
    InvalidCoordination(String),
}
