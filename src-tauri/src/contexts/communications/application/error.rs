use crate::contexts::communications::domain::CommunicationsDomainError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommunicationsApplicationError {
    Domain(CommunicationsDomainError),
    Failure {
        safe_code: String,
        user_message: Option<String>,
    },
}

impl CommunicationsApplicationError {
    pub(crate) fn failure(safe_code: impl Into<String>) -> Self {
        Self::Failure {
            safe_code: safe_code.into(),
            user_message: None,
        }
    }

    pub(crate) fn user_visible(
        safe_code: impl Into<String>,
        user_message: impl Into<String>,
    ) -> Self {
        Self::Failure {
            safe_code: safe_code.into(),
            user_message: Some(user_message.into()),
        }
    }

    pub(crate) fn safe_code(&self) -> &str {
        match self {
            Self::Domain(_) => "communications-domain-invalid",
            Self::Failure { safe_code, .. } => safe_code,
        }
    }

    pub(crate) fn user_message(&self) -> Option<&str> {
        match self {
            Self::Domain(_) => None,
            Self::Failure { user_message, .. } => user_message.as_deref(),
        }
    }
}

impl From<CommunicationsDomainError> for CommunicationsApplicationError {
    fn from(error: CommunicationsDomainError) -> Self {
        Self::Domain(error)
    }
}

impl fmt::Display for CommunicationsApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(formatter),
            Self::Failure {
                safe_code,
                user_message,
            } => formatter.write_str(user_message.as_deref().unwrap_or(safe_code)),
        }
    }
}

impl std::error::Error for CommunicationsApplicationError {}
