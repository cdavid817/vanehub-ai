// See `identity.rs` for why this lands ahead of its production caller.
#![cfg_attr(not(test), allow(dead_code))]

//! Whether a connector subject currently has a definition to connect with.
//!
//! The authority is the active snapshot, reached through `extension_platform`'s published API:
//!
//! ```text
//! Installation -> Active Generation Pointer -> Runtime Generation -> Snapshot
//! ```
//!
//! **Not "the most recently recorded revision".** That answer is wrong in two ways: a version
//! recorded by an install that has not activated leads it, and after a rollback from v2 to v1 the
//! abandoned v2 still leads it. Either way a connector would be described — and, once the connect
//! path exists, dialled — against a definition the platform is not running.
//!
//! This is a read that computes a state, not a write that stores one. Nothing here deletes an
//! instance, drops a binding, clears a credential handle, or rebinds anything: the failure mode of
//! a read is a wrong report, and the failure mode of a write is a user's configured credential
//! disappearing because a projection was briefly behind.

use super::{ConnectorDefinitionDigest, ConnectorGlobalId, ConnectorSnapshotRef};

/// What the platform runs for this connector's contribution.
///
/// This subdomain's own vocabulary, mirrored from `extension_platform`'s answer, so the port is
/// consumer-owned and a change to the platform's enum cannot silently retype this one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActiveConnectorSnapshot {
    /// The extension contributing this connector is running `snapshot`, which declares `declared`.
    ///
    /// `declared` is `None` when the running snapshot does not contribute the connector at all, or
    /// contributes it without a recorded digest. Both mean there is nothing to connect with.
    Running {
        snapshot: ConnectorSnapshotRef,
        declared: Option<ConnectorDefinitionDigest>,
    },
    /// An installation contributes it, but no generation is active. Installed, not running.
    NoActiveGeneration,
    /// Nothing installed contributes it.
    NotInstalled,
    /// The platform could not be asked.
    ///
    /// Every path from here is `Unavailable`: a subdomain that cannot reach the authority does not
    /// get to conclude that a connector is ready to dial.
    Unknown,
}

/// Everything a verdict is computed from. Gathered by the caller; nothing here reads storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectorFacts {
    pub(crate) connector: ConnectorGlobalId,
    pub(crate) active: ActiveConnectorSnapshot,
    /// The definition recorded for exactly `(active snapshot, connector)`.
    pub(crate) recorded_at_active: Option<ConnectorDefinitionDigest>,
    /// Whether the subject has any recorded revision at all. Separates "uninstalled, with the
    /// evidence of what it was still here" from "a subject that exists only because an instance
    /// mentions it".
    pub(crate) has_any_revision: bool,
}

/// The state of one subject relative to what the platform is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ConnectorReadiness {
    /// The active snapshot declares the connector and the recorded definition agrees.
    Ready,
    /// The subject has recorded definitions, but nothing installed contributes it any more.
    Orphaned,
    /// Nothing to connect with right now: no active generation, the active snapshot does not
    /// declare it, no definition is recorded for that snapshot, or the platform is unreachable.
    Unavailable,
    /// The active snapshot declares it and the recorded definition disagrees.
    Drifted,
}

impl ConnectorReadiness {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Orphaned => "orphaned",
            Self::Unavailable => "unavailable",
            Self::Drifted => "drifted",
        }
    }

    /// Whether a new connect or execute may start.
    ///
    /// The only thing readiness gates. Existing instances, bindings, and credential handles are
    /// untouched by every one of the other three — see `instance.rs`.
    pub(crate) const fn admits_connect(self) -> bool {
        matches!(self, Self::Ready)
    }
}

pub(crate) const ALL_CONNECTOR_READINESS: &[ConnectorReadiness] = &[
    ConnectorReadiness::Ready,
    ConnectorReadiness::Orphaned,
    ConnectorReadiness::Unavailable,
    ConnectorReadiness::Drifted,
];

/// A subject and what reconciliation concluded about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectorVerdict {
    pub(crate) connector: ConnectorGlobalId,
    pub(crate) readiness: ConnectorReadiness,
    /// The snapshot the platform is running, when the verdict is about one.
    pub(crate) snapshot: Option<ConnectorSnapshotRef>,
}

/// Computes one subject's readiness. Pure; the caller supplies what it gathered.
pub(crate) fn judge_connector(facts: &ConnectorFacts) -> ConnectorVerdict {
    let verdict = |readiness, snapshot| ConnectorVerdict {
        connector: facts.connector.clone(),
        readiness,
        snapshot,
    };

    match &facts.active {
        ActiveConnectorSnapshot::Unknown | ActiveConnectorSnapshot::NoActiveGeneration => {
            verdict(ConnectorReadiness::Unavailable, None)
        }
        ActiveConnectorSnapshot::NotInstalled if facts.has_any_revision => {
            verdict(ConnectorReadiness::Orphaned, None)
        }
        ActiveConnectorSnapshot::NotInstalled => verdict(ConnectorReadiness::Unavailable, None),
        ActiveConnectorSnapshot::Running { snapshot, declared } => {
            let readiness = match (declared, &facts.recorded_at_active) {
                (Some(declared), Some(recorded)) if declared == recorded => {
                    ConnectorReadiness::Ready
                }
                (Some(_), Some(_)) => ConnectorReadiness::Drifted,
                // Either the running snapshot does not declare this connector, or nothing is
                // recorded for it here. Either way there is nothing to connect with, and the one
                // thing that must not happen is reaching for a revision from another snapshot.
                _ => ConnectorReadiness::Unavailable,
            };
            let named = matches!(
                readiness,
                ConnectorReadiness::Ready | ConnectorReadiness::Drifted
            )
            .then(|| snapshot.clone());
            verdict(readiness, named)
        }
    }
}
