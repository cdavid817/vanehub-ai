use crate::contexts::personalization::domain::{
    PersonalizationDomainError, ResetRefusal, RevisionConflict,
};

/// What the personalization application layer can return.
///
/// `RevisionConflict` is deliberately its own variant rather than a `Storage` string: the caller
/// has to be able to preserve the user's draft and offer a reload, and it cannot do that if a
/// conflict is indistinguishable from a disk error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PersonalizationApplicationError {
    Domain(PersonalizationDomainError),
    RevisionConflict(RevisionConflict),
    NotFound,
    /// A project-scoped operation was requested without a resolvable workspace.
    WorkspaceRequired,
    ResetRefused(ResetRefusal),
    /// Persistence failed. Carries a message for local diagnostics; the command layer maps this to
    /// a safe typed error rather than forwarding filesystem or SQLite text to the UI.
    Storage(String),
    /// Migration or reconciliation has not established a safe generation.
    MaintenanceRequired,
}

impl From<PersonalizationDomainError> for PersonalizationApplicationError {
    fn from(error: PersonalizationDomainError) -> Self {
        Self::Domain(error)
    }
}

impl From<RevisionConflict> for PersonalizationApplicationError {
    fn from(conflict: RevisionConflict) -> Self {
        Self::RevisionConflict(conflict)
    }
}

impl std::fmt::Display for PersonalizationApplicationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Domain(error) => write!(formatter, "{error}"),
            Self::RevisionConflict(conflict) => write!(
                formatter,
                "This item changed since it was loaded (expected revision {}, current {}).",
                conflict.expected, conflict.current
            ),
            Self::NotFound => write!(
                formatter,
                "The requested personalization record no longer exists."
            ),
            Self::WorkspaceRequired => {
                write!(formatter, "This operation requires a resolvable workspace.")
            }
            Self::ResetRefused(refusal) => {
                write!(formatter, "The reset was not authorized: {refusal:?}.")
            }
            Self::Storage(message) => {
                write!(formatter, "Personalization storage failed: {message}")
            }
            Self::MaintenanceRequired => write!(
                formatter,
                "Personalization data is being migrated or repaired and is temporarily unavailable."
            ),
        }
    }
}
