use crate::contexts::desktop::domain::FloatingAssistantDomainError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FloatingAssistantApplicationError {
    Domain(FloatingAssistantDomainError),
    Repository(String),
    Window(String),
}

impl fmt::Display for FloatingAssistantApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(formatter),
            Self::Repository(message) => {
                write!(formatter, "floating assistant repository error: {message}")
            }
            Self::Window(message) => {
                write!(formatter, "floating assistant window error: {message}")
            }
        }
    }
}

impl std::error::Error for FloatingAssistantApplicationError {}

impl From<FloatingAssistantDomainError> for FloatingAssistantApplicationError {
    fn from(error: FloatingAssistantDomainError) -> Self {
        Self::Domain(error)
    }
}
