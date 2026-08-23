// The report that surfaces this lands with Task Group 7; Task Group 3 lands the computation.
#![cfg_attr(not(test), allow(dead_code))]

//! Reconciling Hook subjects against the snapshots their definitions name.
//!
//! `snapshot_id` on a definition revision carries no foreign key — `extension_platform` owns
//! snapshots, and an enforced reference would couple two subdomains' storage and let one's
//! deletions reach into the other's evidence. This makes up the difference, and it is a **read
//! that computes a state, not a write that stores one**.
//!
//! Nothing here deletes a row, rebinds anything, or activates anything. The failure mode of a read
//! is a wrong report; the failure mode of a write is a user's enablement quietly disappearing
//! because a projection was momentarily behind, and only one of those is recoverable.
//!
//! A stored `orphaned` mark would be worse still: it goes stale the moment the snapshot comes
//! back, and is then a lie that outlives its cause.

use super::{HookDefinitionRepository, HookSubjectRepository, SnapshotProjectionPort};
use crate::contexts::tooling::lifecycle_hooks::domain::{
    judge_subject, HookGlobalId, SubjectProjection, SubjectVerdict,
};

/// Every subject, and what this installation currently knows about it.
///
/// Ordered by subject id so two runs against the same database produce the same report — a
/// reconciliation that reshuffled would make a stored diagnostic unmatchable.
pub(crate) fn reconcile_subjects(
    subjects: &dyn HookSubjectRepository,
    definitions: &dyn HookDefinitionRepository,
    projection: &dyn SnapshotProjectionPort,
) -> Result<Vec<SubjectVerdict>, String> {
    let mut verdicts = Vec::new();
    for subject in subjects.all()? {
        verdicts.push(reconcile_subject(&subject.hook, definitions, projection)?);
    }
    Ok(verdicts)
}

/// One subject's readiness.
///
/// The revision considered is the most recently recorded one. A subject with several — an upgrade
/// records beside the old revision rather than over it — is judged on the one it would dispatch
/// from, because that is the question the verdict answers.
pub(crate) fn reconcile_subject(
    hook: &HookGlobalId,
    definitions: &dyn HookDefinitionRepository,
    projection: &dyn SnapshotProjectionPort,
) -> Result<SubjectVerdict, String> {
    let latest = definitions.revisions(hook)?.into_iter().next();
    let projected = SubjectProjection {
        hook: hook.clone(),
        revision: latest
            .as_ref()
            .map(|revision| (revision.snapshot.clone(), revision.digest.clone())),
    };

    let fact = match &projected.revision {
        Some((snapshot, _)) => projection.fact(hook, snapshot)?,
        // Nothing to look up: the subject names no snapshot, which the domain reads as
        // `Unavailable` without needing the projection's opinion.
        None => None,
    };

    Ok(judge_subject(&projected, fact.as_ref()))
}
