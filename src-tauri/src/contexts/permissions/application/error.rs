use crate::contexts::permissions::domain::PermissionsDomainError;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum PermissionsApplicationError {
    #[error("{0}")]
    NotFound(String),
    #[error(transparent)]
    Domain(#[from] PermissionsDomainError),
    #[error("{message}")]
    Infrastructure {
        category: &'static str,
        message: String,
    },
    #[error("{0}")]
    Internal(String),
}

impl PermissionsApplicationError {
    pub(crate) fn infrastructure(category: &'static str, message: impl Into<String>) -> Self {
        Self::Infrastructure {
            category,
            message: message.into(),
        }
    }
}
