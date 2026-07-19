use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub(crate) enum ApplicationError {
    #[error("{0}")]
    NotFound(String),
    #[error("{message}")]
    Infrastructure {
        category: &'static str,
        message: String,
    },
    #[error("{0}")]
    Internal(String),
}

impl ApplicationError {
    pub(crate) fn infrastructure(
        category: &'static str,
        command_safe_message: impl Into<String>,
    ) -> Self {
        Self::Infrastructure {
            category,
            message: command_safe_message.into(),
        }
    }
}
