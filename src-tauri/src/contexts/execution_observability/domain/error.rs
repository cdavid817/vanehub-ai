use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum ExecutionDomainError {
    #[error("{kind} must be {expected} lowercase hexadecimal characters and cannot be all zero")]
    InvalidTelemetryId { kind: &'static str, expected: usize },
    #[error("execution run id must be a non-empty UUID-shaped value")]
    InvalidRunId,
    #[error("safe attribute key must contain 1 to {max} characters")]
    InvalidAttributeKey { max: usize },
    #[error("safe string attribute must contain at most {max} characters")]
    AttributeValueTooLong { max: usize },
    #[error("safe attributes must contain at most {max} entries")]
    TooManyAttributes { max: usize },
    #[error("page size must be between 1 and {max}")]
    InvalidPageSize { max: usize },
    #[error("page token must contain at most {max} characters")]
    InvalidPageToken { max: usize },
    #[error("execution timestamp is required")]
    TimestampRequired,
    #[error("execution span name must contain 1 to {max} characters")]
    InvalidSpanName { max: usize },
    #[error("invalid observability setting '{field}': {message}")]
    InvalidSetting {
        field: &'static str,
        message: &'static str,
    },
}
