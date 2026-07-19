use crate::contexts::tooling::extensions::domain::ExtensionDomainError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExtensionApplicationError {
    Domain(ExtensionDomainError),
    Repository(String),
    Installation(String),
    Runtime(String),
    Operation(String),
    Logging(String),
    ConcurrentMutation(String),
}

impl fmt::Display for ExtensionApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(formatter),
            Self::Repository(message)
            | Self::Installation(message)
            | Self::Runtime(message)
            | Self::Operation(message)
            | Self::Logging(message)
            | Self::ConcurrentMutation(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ExtensionApplicationError {}

impl From<ExtensionDomainError> for ExtensionApplicationError {
    fn from(error: ExtensionDomainError) -> Self {
        Self::Domain(error)
    }
}
