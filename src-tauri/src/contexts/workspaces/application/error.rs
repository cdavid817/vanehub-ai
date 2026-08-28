use crate::contexts::workspaces::domain::WorkspaceDomainError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkspaceApplicationError {
    Domain(WorkspaceDomainError),
    Validation(String),
    /// The request was well formed and the state moved under it.
    ///
    /// Carries a stable reason code and nothing else, because the code is what a caller matches
    /// on. A sentence with the code inside it would make every caller parse prose to find it, and
    /// the first one to get the parsing wrong would show the wrong message with no way to tell.
    Conflict(&'static str),
    Repository(String),
    Selection(String),
    Filesystem(String),
    Storage(String),
    LaunchFailed(String),
    SessionNotFound(String),
    PolicyDenied {
        session_id: String,
        action: String,
    },
}

impl fmt::Display for WorkspaceApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(formatter),
            Self::Validation(message) => formatter.write_str(message),
            Self::Conflict(code) => formatter.write_str(code),
            Self::Repository(message) => write!(formatter, "workspace repository error: {message}"),
            Self::Selection(message) => write!(formatter, "workspace selection error: {message}"),
            Self::Filesystem(message) => write!(formatter, "workspace filesystem error: {message}"),
            Self::Storage(message) => write!(formatter, "workspace storage error: {message}"),
            Self::LaunchFailed(message) => write!(formatter, "workspace launch failed: {message}"),
            Self::SessionNotFound(session_id) => {
                write!(formatter, "workspace session not found: {session_id}")
            }
            Self::PolicyDenied { session_id, action } => write!(
                formatter,
                "Verifier session {session_id} cannot perform workspace action: {action}"
            ),
        }
    }
}

impl std::error::Error for WorkspaceApplicationError {}

impl From<WorkspaceDomainError> for WorkspaceApplicationError {
    fn from(error: WorkspaceDomainError) -> Self {
        Self::Domain(error)
    }
}
