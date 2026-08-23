// See `identity.rs` for why this lands ahead of its production caller.
#![cfg_attr(not(test), allow(dead_code))]

//! What one connector is, in one snapshot — and what a second recording of the same pair means.
//!
//! The same split as lifecycle Hooks, for the same reasons. A **subject** is the stable
//! `<connector-global-id>`; a **definition revision** is `(snapshot, subject)` and is immutable; an
//! **instance** references the subject. An instance pinned to a versioned definition would be
//! orphaned by every upgrade, would vanish while a definition was momentarily unavailable — taking
//! a user's credential handle with it — and would be resurrected with whatever configuration an
//! older snapshot happened to ship when rolled back.
//!
//! Recording is idempotent for the *same* definition and refused for a different one. `(snapshot,
//! subject)` naming two digests is two incompatible answers to what the connector is in that
//! snapshot, and taking the later one would let a rebuild change what an already-installed
//! snapshot means.

use super::{ConnectorDefinitionDigest, ConnectorGlobalId, ConnectorSnapshotRef, OwnerExtensionId};

/// The stable identity an instance attaches to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectorSubject {
    pub(crate) connector: ConnectorGlobalId,
    /// Which extension contributes it. Recorded so an operator can find the package to uninstall;
    /// it carries no foreign key, because `extension_platform` owns extensions.
    pub(crate) owner_extension: OwnerExtensionId,
    pub(crate) first_seen_at: String,
}

/// What one connector is, in one snapshot. Immutable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectorDefinitionRevision {
    pub(crate) snapshot: ConnectorSnapshotRef,
    pub(crate) connector: ConnectorGlobalId,
    pub(crate) digest: ConnectorDefinitionDigest,
    pub(crate) recorded_at: String,
}

/// What recording a revision would mean, given whatever already holds the pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConnectorDefinitionOutcome {
    /// Nothing was recorded for this pair. The revision binds it.
    Recorded,
    /// The same definition, recorded again. Reinstalling a snapshot is not a conflict.
    AlreadyRecorded,
    /// The pair is bound to a different definition. Refused; both digests are reported.
    Conflict(ConnectorDefinitionContentConflict),
}

impl ConnectorDefinitionOutcome {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Recorded => "connector_definition_recorded",
            Self::AlreadyRecorded => "connector_definition_already_recorded",
            Self::Conflict(_) => "connector_definition_content_conflict",
        }
    }

    /// Whether the connector may be connected from this snapshot.
    ///
    /// A conflicted pair has two answers to what the connector is, and connecting on either is a
    /// guess about which endpoint and which scopes were reviewed.
    pub(crate) const fn admits_connect(&self) -> bool {
        matches!(self, Self::Recorded | Self::AlreadyRecorded)
    }
}

/// The same `(snapshot, subject)`, twice, with different definitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectorDefinitionContentConflict {
    pub(crate) recorded_digest: ConnectorDefinitionDigest,
    pub(crate) offered_digest: ConnectorDefinitionDigest,
    pub(crate) recorded_at: String,
}

impl ConnectorDefinitionContentConflict {
    pub(crate) const fn code(&self) -> &'static str {
        "connector_definition_content_conflict"
    }
}

/// Decides what recording a revision means against whatever already holds the pair.
///
/// A pure comparison, so the rule lives in one place and a repository only has to say what it
/// found.
pub(crate) fn decide_connector_definition(
    offered: &ConnectorDefinitionRevision,
    recorded: Option<&ConnectorDefinitionRevision>,
) -> ConnectorDefinitionOutcome {
    let Some(recorded) = recorded else {
        return ConnectorDefinitionOutcome::Recorded;
    };
    if recorded.digest == offered.digest {
        return ConnectorDefinitionOutcome::AlreadyRecorded;
    }
    ConnectorDefinitionOutcome::Conflict(ConnectorDefinitionContentConflict {
        recorded_digest: recorded.digest.clone(),
        offered_digest: offered.digest.clone(),
        recorded_at: recorded.recorded_at.clone(),
    })
}
