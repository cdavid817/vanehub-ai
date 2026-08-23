// The reconciliation report that surfaces these lands with Task Group 7; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Whether a Hook subject currently has something to run.
//!
//! ## The authority is the active snapshot, and nothing else
//!
//! A subject can have several recorded definition revisions at once — an upgrade records a new one
//! beside the old rather than over it, which is what lets a rollback still have something to
//! dispatch from. So "which revision is in force" is a real question, and **"the most recently
//! recorded one" is the wrong answer twice over**:
//!
//! * a version that has been recorded but not yet activated would win it, so the report would
//!   claim a Hook is running code the platform has not activated;
//! * after a rollback from v2 to v1, v2 is still the most recently *recorded* revision, so the
//!   report would name v2 while the platform runs v1.
//!
//! The authoritative chain lives in `extension_platform`:
//!
//! ```text
//! Installation -> Active Generation Pointer -> Runtime Generation -> Snapshot
//! ```
//!
//! and this subdomain reaches it through a port, never by joining another subdomain's tables. The
//! revision consulted is the one recorded for exactly `(active snapshot, hook)`. Ordering by
//! `recorded_at` survives only as a diagnostic listing; it takes no part in a readiness verdict.
//!
//! ## Reading, not writing
//!
//! `snapshot_id` on a definition revision carries no foreign key, so the database cannot enforce
//! it. This makes up the difference, and it is a **read that computes a state, not a write that
//! stores one**. A stored `orphaned` mark goes stale the moment the extension is reinstalled and
//! is then a lie that outlives its cause; worse, a reconciliation that writes is one that can
//! delete or rebind by mistake. The failure mode of a read is a wrong report. Nothing here deletes
//! a row, rebinds anything, or activates anything.

use super::{DefinitionDigest, HookGlobalId, SnapshotRef};

/// What the platform runs for this Hook's contribution id.
///
/// Mirrors `extension_platform`'s answer in this subdomain's own vocabulary, so the port is
/// consumer-owned and a change to the platform's enum cannot silently retype this one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActiveSnapshot {
    /// The extension contributing this Hook is running `snapshot`, which declares `declared`.
    ///
    /// `declared` is `None` when the running snapshot does not contribute this Hook at all, or
    /// contributes it without a recorded digest. Both mean there is nothing to dispatch.
    Running {
        snapshot: SnapshotRef,
        declared: Option<DefinitionDigest>,
    },
    /// An installation contributes this Hook, but no generation is active. Installed, not running.
    NoActiveGeneration,
    /// Nothing installed contributes this Hook.
    NotInstalled,
    /// The platform could not be asked.
    ///
    /// Answered conservatively rather than optimistically: a subdomain that cannot reach the
    /// authority does not get to guess that a Hook is ready. Every path from here is
    /// `Unavailable`.
    Unknown,
}

/// Everything a verdict is computed from. Gathered by the caller; nothing here reads storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubjectFacts {
    pub(crate) hook: HookGlobalId,
    pub(crate) active: ActiveSnapshot,
    /// The definition recorded for exactly `(active snapshot, hook)`.
    ///
    /// Meaningful only when `active` is `Running`; the caller does not look it up otherwise,
    /// because there is no snapshot to look it up against.
    pub(crate) recorded_at_active: Option<DefinitionDigest>,
    /// Whether the subject has any recorded revision at all.
    ///
    /// Separates "uninstalled, and the evidence of what it was is still here" from "this subject
    /// exists only because a binding or an execution mentions it".
    pub(crate) has_any_revision: bool,
}

/// The state of one subject relative to what the platform is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum SubjectReadiness {
    /// The active snapshot declares this Hook and the recorded definition agrees with it.
    Ready,
    /// The subject has recorded definitions, but nothing installed contributes it any more.
    Orphaned,
    /// Nothing to dispatch right now: no active generation, or the active snapshot does not
    /// declare this Hook, or the subject has no definition at all.
    Unavailable,
    /// The active snapshot declares this Hook and the recorded definition disagrees with it.
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
    /// Three of the four are not failures to repair, they are reasons to stand down. Dispatching a
    /// drifted definition is the worst of them: it runs something other than what was installed,
    /// while the operator believes the reviewed version is in effect.
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
    /// The snapshot the platform is running, when there is one. Absent whenever the verdict is not
    /// about a running snapshot, which is every case except `Ready` and `Drifted`.
    pub(crate) snapshot: Option<SnapshotRef>,
}

/// Computes one subject's readiness. Pure; the caller supplies what it gathered.
pub(crate) fn judge_subject(facts: &SubjectFacts) -> SubjectVerdict {
    let verdict = |readiness, snapshot| SubjectVerdict {
        hook: facts.hook.clone(),
        readiness,
        snapshot,
    };

    match &facts.active {
        // The platform is unreachable. Not knowing is not the same as being ready, and a report
        // that guessed would be worse than one that says it cannot tell.
        ActiveSnapshot::Unknown | ActiveSnapshot::NoActiveGeneration => {
            verdict(SubjectReadiness::Unavailable, None)
        }
        ActiveSnapshot::NotInstalled if facts.has_any_revision => {
            verdict(SubjectReadiness::Orphaned, None)
        }
        ActiveSnapshot::NotInstalled => verdict(SubjectReadiness::Unavailable, None),
        ActiveSnapshot::Running { snapshot, declared } => {
            let readiness = match (declared, &facts.recorded_at_active) {
                (Some(declared), Some(recorded)) if declared == recorded => SubjectReadiness::Ready,
                (Some(_), Some(_)) => SubjectReadiness::Drifted,
                // Either the running snapshot does not declare this Hook, or this subdomain has
                // nothing recorded for it. Either way there is nothing to dispatch, and the one
                // thing that must not happen is reaching for a revision from another snapshot.
                _ => SubjectReadiness::Unavailable,
            };
            let named = matches!(
                readiness,
                SubjectReadiness::Ready | SubjectReadiness::Drifted
            )
            .then(|| snapshot.clone());
            verdict(readiness, named)
        }
    }
}
