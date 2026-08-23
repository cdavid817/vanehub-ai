// The install and activation flows that make these answers non-trivial land with Task Group 4.
#![cfg_attr(not(test), allow(dead_code))]

//! What the platform currently runs for one contribution.
//!
//! Published so that Hooks, Connectors, and the rule sources can ask "is this contribution live,
//! and what does the live snapshot say it is" without reading installations, generations, or
//! snapshots themselves. The authority chain is
//!
//! ```text
//! Installation -> Active Generation Pointer -> Runtime Generation -> Snapshot
//! ```
//!
//! and it is the *only* chain that answers the question. "The most recently recorded definition"
//! is not an answer: a version that has been recorded but not activated would win it, and so would
//! the newer version after a rollback to an older one. Both cases end with a consumer dispatching
//! something the platform is not running.
//!
//! The identifiers cross the boundary as plain text on purpose. A consumer validates them with its
//! own value types, so neither side has to adopt the other's, and neither can be handed an
//! identifier that only type-checks because the two happen to share a crate.

/// What the platform runs for one contribution id, right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActiveContribution {
    /// An installation contributes this id and is running `snapshot_id`.
    Running {
        snapshot_id: String,
        /// What the running snapshot declares for this contribution.
        ///
        /// `None` covers two cases that are the same answer for a consumer: the running snapshot
        /// does not contribute this id at all (the newer version dropped it), and it contributes
        /// it without a recorded digest (a row written before contributions carried one). Both
        /// mean there is nothing to dispatch, so neither may read as ready.
        declared_digest: Option<String>,
    },
    /// An installation contributes this id, but no generation is active. Installed, not running.
    NoActiveGeneration,
    /// Nothing installed contributes this id.
    NotInstalled,
}

impl ActiveContribution {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Running { .. } => "contribution_running",
            Self::NoActiveGeneration => "contribution_no_active_generation",
            Self::NotInstalled => "contribution_not_installed",
        }
    }
}

/// Why the platform could not say what it runs for a contribution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActiveContributionError {
    /// Two installations claim the same contribution id.
    ///
    /// Prevented upstream — a global contribution id is namespaced by its extension, and admission
    /// refuses one extension claiming another's namespace — and not prevented by the database,
    /// whose key is `(snapshot_id, global_id)`. Reported rather than resolved: picking one of two
    /// owners would silently dispatch an extension the operator did not install for that id.
    AmbiguousOwner,
    Storage(String),
}

impl ActiveContributionError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::AmbiguousOwner => "ambiguous_contribution_owner",
            Self::Storage(_) => "active_contribution_storage_failure",
        }
    }
}

pub(crate) fn all_active_contribution_errors() -> Vec<ActiveContributionError> {
    vec![
        ActiveContributionError::AmbiguousOwner,
        ActiveContributionError::Storage(String::new()),
    ]
}
