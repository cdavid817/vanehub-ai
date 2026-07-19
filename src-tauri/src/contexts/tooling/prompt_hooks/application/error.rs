use crate::contexts::tooling::prompt_hooks::domain::PromptHookDomainError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PromptHookApplicationError {
    Domain(PromptHookDomainError),
    NotFound(String),
    Conflict(String),
    Repository(String),
}

impl fmt::Display for PromptHookApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(formatter),
            Self::NotFound(hook_id) => write!(formatter, "Prompt Hook not found: {hook_id}"),
            Self::Conflict(hook_id) => write!(formatter, "Prompt Hook already exists: {hook_id}"),
            Self::Repository(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for PromptHookApplicationError {}

impl From<PromptHookDomainError> for PromptHookApplicationError {
    fn from(error: PromptHookDomainError) -> Self {
        Self::Domain(error)
    }
}
