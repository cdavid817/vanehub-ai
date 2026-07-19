use crate::contexts::sessions::domain::SessionsDomainError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionsApplicationError {
    Domain(SessionsDomainError),
    Validation(String),
    AgentNotFound(String),
    UnsupportedInteractionMode(String),
    SessionNotFound(String),
    MessageNotFound(String),
    CategoryNotFound(String),
    CategoryNameConflict(String),
    Repository(String),
    Transaction(String),
    FileContent(String),
    Operation(String),
    Logging(String),
    Serialization(String),
    Workspace(String),
    WorkspaceLaunch(String),
    Runtime(String),
    RuntimeLaunch(String),
}

impl fmt::Display for SessionsApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(formatter),
            Self::Validation(message) => formatter.write_str(message),
            Self::AgentNotFound(agent_id) => write!(formatter, "Agent not found: {agent_id}"),
            Self::UnsupportedInteractionMode(mode) => {
                write!(formatter, "Unsupported interaction mode: {mode}")
            }
            Self::SessionNotFound(session_id) => {
                write!(formatter, "Session not found: {session_id}")
            }
            Self::MessageNotFound(message_id) => {
                write!(formatter, "Message not found: {message_id}")
            }
            Self::CategoryNotFound(category_id) => {
                write!(formatter, "Session category not found: {category_id}")
            }
            Self::CategoryNameConflict(name) => {
                write!(formatter, "Session category name already exists: {name}")
            }
            Self::Repository(message) => write!(formatter, "session repository error: {message}"),
            Self::Transaction(message) => {
                write!(formatter, "session transaction error: {message}")
            }
            Self::FileContent(message) => {
                write!(formatter, "session file-content error: {message}")
            }
            Self::Operation(message) => write!(formatter, "session operation error: {message}"),
            Self::Logging(message) => write!(formatter, "session logging error: {message}"),
            Self::Serialization(message) => {
                write!(formatter, "session serialization error: {message}")
            }
            Self::Workspace(message) => write!(formatter, "session workspace error: {message}"),
            Self::WorkspaceLaunch(message) => {
                write!(formatter, "session workspace launch error: {message}")
            }
            Self::Runtime(message) => write!(formatter, "session runtime error: {message}"),
            Self::RuntimeLaunch(message) => {
                write!(formatter, "session runtime launch error: {message}")
            }
        }
    }
}

impl std::error::Error for SessionsApplicationError {}

impl From<SessionsDomainError> for SessionsApplicationError {
    fn from(error: SessionsDomainError) -> Self {
        Self::Domain(error)
    }
}
