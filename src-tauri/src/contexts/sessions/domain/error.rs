use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArchivedSessionAction {
    Activate,
    SendMessage,
    StartGeneration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionsDomainError {
    IdentityRequired(&'static str),
    IdentityContainsControl(&'static str),
    SessionTitleRequired,
    CategoryNameRequired,
    ConnectorRequired,
    ConnectorCannotActivate,
    ArchivedSession {
        session_id: String,
        action: ArchivedSessionAction,
    },
    InvalidMessageRole(String),
    InvalidMessageStatus(String),
    InvalidMessageTransition {
        from: &'static str,
        to: &'static str,
    },
    MessageOwnershipMismatch {
        message_id: String,
        expected_session_id: String,
        actual_session_id: String,
    },
    FileReferenceFieldRequired(&'static str),
    InvalidFileReferenceSize,
    DuplicateFileReferencePath(String),
    TooManyFileReferences,
    UnsupportedChatAgent(String),
    UnsupportedPermissionMode,
    ProviderMismatch {
        provider_id: String,
        agent_id: String,
    },
    UnsupportedModel {
        model_id: String,
        agent_id: String,
    },
    UnsupportedReasoningDepth,
}

impl fmt::Display for SessionsDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdentityRequired(kind) => write!(formatter, "{kind} cannot be empty."),
            Self::IdentityContainsControl(kind) => {
                write!(formatter, "{kind} contains invalid control characters.")
            }
            Self::SessionTitleRequired => formatter.write_str("Session title cannot be empty."),
            Self::CategoryNameRequired => formatter.write_str("Category name cannot be empty."),
            Self::ConnectorRequired => {
                formatter.write_str("Connector-owned sessions require a connector id.")
            }
            Self::ConnectorCannotActivate => {
                formatter.write_str("Connector-owned sessions cannot replace the active session.")
            }
            Self::ArchivedSession { session_id, action } => match action {
                ArchivedSessionAction::Activate => {
                    write!(formatter, "Cannot switch to archived session: {session_id}")
                }
                ArchivedSessionAction::SendMessage => {
                    write!(formatter, "Cannot send a message to archived session: {session_id}")
                }
                ArchivedSessionAction::StartGeneration => {
                    write!(formatter, "Cannot start generation for archived session: {session_id}")
                }
            },
            Self::InvalidMessageRole(role) => write!(formatter, "Unsupported message role: {role}"),
            Self::InvalidMessageStatus(status) => {
                write!(formatter, "Unsupported message status: {status}")
            }
            Self::InvalidMessageTransition { from, to } => {
                write!(formatter, "Message cannot transition from {from} to {to}.")
            }
            Self::MessageOwnershipMismatch {
                message_id,
                expected_session_id,
                actual_session_id,
            } => write!(
                formatter,
                "Message {message_id} belongs to session {actual_session_id}, not {expected_session_id}."
            ),
            Self::FileReferenceFieldRequired(field) => {
                write!(formatter, "File reference {field} cannot be empty.")
            }
            Self::InvalidFileReferenceSize => {
                formatter.write_str("File reference size cannot be negative.")
            }
            Self::DuplicateFileReferencePath(path) => {
                write!(formatter, "File reference path is duplicated: {path}")
            }
            Self::TooManyFileReferences => {
                formatter.write_str("At most 5 files can be referenced in one message.")
            }
            Self::UnsupportedChatAgent(agent_id) => {
                write!(formatter, "Unsupported chat agent: {agent_id}.")
            }
            Self::UnsupportedPermissionMode => {
                formatter.write_str("Unsupported permission mode.")
            }
            Self::ProviderMismatch {
                provider_id,
                agent_id,
            } => write!(
                formatter,
                "Provider '{provider_id}' does not match session agent '{agent_id}'."
            ),
            Self::UnsupportedModel { model_id, agent_id } => write!(
                formatter,
                "Model '{model_id}' is unsupported for session agent '{agent_id}'."
            ),
            Self::UnsupportedReasoningDepth => {
                formatter.write_str("Unsupported reasoning depth.")
            }
        }
    }
}

impl std::error::Error for SessionsDomainError {}
