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
    /// Not constructed by any current code path — kept for exhaustive error mapping at the
    /// command boundary (`commands/error.rs`) ahead of a producer that needs it.
    #[allow(dead_code)]
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
