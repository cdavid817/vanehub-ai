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
    /// A pre-governance caller addressed a memory by a display name that now matches more than one
    /// record, and no legacy-identity alias resolves the ambiguity.
    ///
    /// Refusing is the only safe answer. Picking the first, the newest, or any sorted position
    /// would silently overwrite one of the user's memories, which is exactly the failure mode
    /// immutable ids were introduced to end.
    AmbiguousLegacyName {
        matches: usize,
    },
    /// Another holder — in this process or another one — owns the memory-directory lock.
    ///
    /// Typed rather than folded into `Storage` because the caller's correct response is entirely
    /// different: retry later, keep long-term memory unavailable, and report a maintenance state,
    /// as opposed to surfacing a disk error.
    MaintenanceBusy,
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
            Self::AmbiguousLegacyName { matches } => write!(
                formatter,
                "This name matches {matches} memories, so it cannot identify one of them. Open the memory you meant and edit it directly."
            ),
            Self::MaintenanceBusy => write!(
                formatter,
                "Another personalization maintenance operation is in progress. Try again shortly."
            ),
        }
    }
}
