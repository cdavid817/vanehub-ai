// The reconciliation report that surfaces these lands with Task Group 7; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Whether a Hook subject currently has something to run.
//!
//! `snapshot_id` on a definition revision is opaque text — `extension_platform` owns snapshots, so
//! the database cannot enforce the reference. This makes up the difference, and it is a **read
//! that computes a state, not a write that stores one**.
//!
//! That distinction is the whole design. A stored `orphaned` mark goes stale the moment the
//! snapshot comes back and is then a lie that outlives its cause. Worse, a reconciliation that
//! writes is a reconciliation that can delete or rebind by mistake: the failure mode of a read is
//! a wrong report, and the failure mode of a write is a user's enablement quietly disappearing
//! because a projection was momentarily behind. So nothing here deletes a row, rebinds anything,
//! or activates anything.
//!
//! The snapshot facts come from `extension_platform`'s published API through a projection port,
//! never from a direct read of another subdomain's tables.

use super::{DefinitionDigest, HookGlobalId, SnapshotRef};

/// What `extension_platform` says about one snapshot, reduced to what this subdomain needs.
///
/// Deliberately tiny: a snapshot has an id and a digest for each contribution it carries, and
/// anything more would be this subdomain knowing things it has no business knowing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SnapshotFact {
    pub(crate) snapshot: SnapshotRef,
    /// The digest the snapshot itself records for this Hook, if it contributes one at all.
    pub(crate) hook_digest: Option<DefinitionDigest>,
}

/// What this subdomain knows about one subject, before reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubjectProjection {
    pub(crate) hook: HookGlobalId,
    /// The definition revision this subject would dispatch from, if it has one.
    pub(crate) revision: Option<(SnapshotRef, DefinitionDigest)>,
}

/// The state of one subject relative to the snapshot its definition names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum SubjectReadiness {
    /// A definition revision exists for a snapshot that exists, and the digests agree.
    Ready,
    /// The snapshot the revision names is gone.
    Orphaned,
    /// The snapshot exists but no definition revision references it, so the subject has a binding
    /// and nothing to bind to right now.
    Unavailable,
    /// Both exist and the stored definition digest disagrees with the snapshot's.
    Drifted,
}

impl SubjectReadiness {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Orphaned => "orphaned",
            Self::Unavailable => "unavailable",
            Self::Drifted => "drifted",
        }
    }

    /// Whether the subject may be dispatched right now.
    ///
    /// Three of the four are not failures to repair, they are reasons to stand down: dispatching a
    /// drifted definition runs something other than what was installed.
    pub(crate) const fn admits_dispatch(self) -> bool {
        matches!(self, Self::Ready)
    }
}

pub(crate) const ALL_SUBJECT_READINESS: &[SubjectReadiness] = &[
    SubjectReadiness::Ready,
    SubjectReadiness::Orphaned,
    SubjectReadiness::Unavailable,
    SubjectReadiness::Drifted,
];

/// A subject and what reconciliation concluded about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubjectVerdict {
    pub(crate) hook: HookGlobalId,
    pub(crate) readiness: SubjectReadiness,
    /// The snapshot the verdict is about, when there is one. Absent for `Unavailable`, which is
    /// precisely the case where the subject names no snapshot.
    pub(crate) snapshot: Option<SnapshotRef>,
}

/// Computes one subject's readiness. Pure; the caller supplies the snapshot fact it looked up.
pub(crate) fn judge_subject(
    projection: &SubjectProjection,
    fact: Option<&SnapshotFact>,
) -> SubjectVerdict {
    let Some((snapshot, digest)) = &projection.revision else {
        // No revision at all: the subject exists because a binding or an execution mentions it,
        // and there is nothing to dispatch. Not an error — an extension that is installed but not
        // activated looks exactly like this.
        return SubjectVerdict {
            hook: projection.hook.clone(),
            readiness: SubjectReadiness::Unavailable,
            snapshot: None,
        };
    };

    let readiness = match fact {
        None => SubjectReadiness::Orphaned,
        // The snapshot is there but does not contribute this Hook. The revision refers to
        // something the snapshot no longer claims, which is the same practical position as an
        // orphan: there is nothing to run.
        Some(fact) => match &fact.hook_digest {
            None => SubjectReadiness::Unavailable,
            Some(current) if current == digest => SubjectReadiness::Ready,
            Some(_) => SubjectReadiness::Drifted,
        },
    };

    SubjectVerdict {
        hook: projection.hook.clone(),
        readiness,
        snapshot: Some(snapshot.clone()),
    }
}
