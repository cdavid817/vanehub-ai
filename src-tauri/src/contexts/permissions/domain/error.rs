use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum PermissionsDomainError {
    #[error("delegation is not enabled; parent_principal_id must be null until a later phase activates it")]
    DelegationNotEnabled,
    #[error("{0} cannot be empty")]
    RequiredValue(&'static str),
}
