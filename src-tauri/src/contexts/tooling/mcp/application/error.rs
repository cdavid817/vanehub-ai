use crate::contexts::tooling::mcp::domain::McpDomainError;
use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub(crate) enum McpApplicationError {
    #[error(transparent)]
    Domain(#[from] McpDomainError),
    #[error("MCP server not found: {0}")]
    ServerNotFound(String),
    #[error("{0}")]
    Validation(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("storage error: {0}")]
    Storage(String),
}
