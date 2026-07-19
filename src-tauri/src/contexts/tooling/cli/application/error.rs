use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub(crate) enum CliApplicationError {
    #[error("{0}")]
    Validation(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("{0}")]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the detection port retains a typed hard-failure category"
        )
    )]
    Detection(String),
    #[error("{0}")]
    Package(String),
    #[error("{0}")]
    Operation(String),
    #[error("{0}")]
    Logging(String),
    #[error("{0}")]
    Internal(String),
}
