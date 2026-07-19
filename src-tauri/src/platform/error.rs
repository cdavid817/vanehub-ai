#![allow(dead_code)]

use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub(crate) enum InfrastructureError {
    #[error("database error: {0}")]
    Database(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("process error: {0}")]
    Process(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("credential error: {0}")]
    Credential(String),
    #[error("serialization error: {0}")]
    Serialization(String),
}

impl InfrastructureError {
    pub(crate) fn category(&self) -> &'static str {
        match self {
            Self::Database(_) => "database",
            Self::Storage(_) => "storage",
            Self::Process(_) => "process",
            Self::Network(_) => "network",
            Self::Credential(_) => "credential",
            Self::Serialization(_) => "serialization",
        }
    }

    pub(crate) fn command_safe_message(&self) -> &'static str {
        match self {
            Self::Database(_) => "The local database operation failed.",
            Self::Storage(_) => "The local storage operation failed.",
            Self::Process(_) => "The external command failed.",
            Self::Network(_) => "The network operation failed.",
            Self::Credential(_) => "The secure credential operation failed.",
            Self::Serialization(_) => "The local data could not be processed.",
        }
    }
}

impl From<rusqlite::Error> for InfrastructureError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error.to_string())
    }
}
