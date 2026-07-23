use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum ExecutionTelemetryError {
    #[error("execution telemetry storage failed: {0}")]
    Storage(String),
    #[error("execution telemetry adapter is unavailable: {0}")]
    Unavailable(String),
    #[error("invalid observability setting '{field}': {message}")]
    InvalidSettings {
        field: &'static str,
        message: &'static str,
    },
}
