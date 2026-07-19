use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SdkApplicationError {
    Validation(String),
    Repository(String),
    Package(String),
    Operation(String),
    Logging(String),
}

impl fmt::Display for SdkApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Validation(message)
            | Self::Repository(message)
            | Self::Package(message)
            | Self::Operation(message)
            | Self::Logging(message) => message,
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for SdkApplicationError {}
