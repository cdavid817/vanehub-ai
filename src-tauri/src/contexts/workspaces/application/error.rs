use crate::contexts::workspaces::domain::WorkspaceDomainError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkspaceApplicationError {
    Domain(WorkspaceDomainError),
    Validation(String),
    Repository(String),
    Selection(String),
    Filesystem(String),
    Storage(String),
    LaunchFailed(String),
    SessionNotFound(String),
}

impl fmt::Display for WorkspaceApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(formatter),
            Self::Validation(message) => formatter.write_str(message),
            Self::Repository(message) => write!(formatter, "workspace repository error: {message}"),
            Self::Selection(message) => write!(formatter, "workspace selection error: {message}"),
            Self::Filesystem(message) => write!(formatter, "workspace filesystem error: {message}"),
            Self::Storage(message) => write!(formatter, "workspace storage error: {message}"),
            Self::LaunchFailed(message) => write!(formatter, "workspace launch failed: {message}"),
            Self::SessionNotFound(session_id) => {
                write!(formatter, "workspace session not found: {session_id}")
            }
        }
    }
}

impl std::error::Error for WorkspaceApplicationError {}

impl From<WorkspaceDomainError> for WorkspaceApplicationError {
    fn from(error: WorkspaceDomainError) -> Self {
        Self::Domain(error)
    }
}
