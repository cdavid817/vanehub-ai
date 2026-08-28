use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum PermissionsDomainError {
    #[error("delegation is not enabled; parent_principal_id must be null until a later phase activates it")]
    DelegationNotEnabled,
    #[error("{0} cannot be empty")]
    RequiredValue(&'static str),
    #[error("a Once scope is never remembered as a grant")]
    UnrememberableScope,
    #[error("only allow and deny can be remembered as a grant effect")]
    UnrememberableEffect,
    #[error("a grant's scope and its recorded owner disagree")]
    ScopeOwnerMismatch,
    #[error("unknown grant activation state")]
    UnknownActivationState,
}
