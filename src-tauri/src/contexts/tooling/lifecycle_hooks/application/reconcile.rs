// The report that surfaces this lands with Task Group 7; Task Group 3 lands the computation.
#![cfg_attr(not(test), allow(dead_code))]

//! Reconciling Hook subjects against the snapshot the platform is actually running.
//!
//! The gathering half of the rule whose deciding half is in `domain::reconciliation`. It asks the
//! platform what is running, then reads the definition recorded for *exactly* that snapshot. It
//! never reads "the latest revision": see the domain module for why that answer is wrong in two
//! separate ways.
//!
//! Nothing here writes. The repositories are used through their reading methods only, and the
//! tests drive fakes that panic on every write method so that stays true.

use super::{ActiveExtensionSnapshotPort, HookDefinitionRepository, HookSubjectRepository};
use crate::contexts::tooling::lifecycle_hooks::domain::{
    judge_subject, ActiveSnapshot, HookDefinitionRevision, HookGlobalId, SubjectFacts,
    SubjectVerdict,
};

/// Every subject, and what this installation currently knows about it.
///
/// Ordered by subject id so two runs against the same database produce the same report — a
/// reconciliation that reshuffled would make a stored diagnostic unmatchable.
pub(crate) fn reconcile_subjects(
    subjects: &dyn HookSubjectRepository,
    definitions: &dyn HookDefinitionRepository,
    active: &dyn ActiveExtensionSnapshotPort,
) -> Result<Vec<SubjectVerdict>, String> {
    let mut verdicts = Vec::new();
    for subject in subjects.all()? {
        verdicts.push(reconcile_subject(&subject.hook, definitions, active)?);
    }
    Ok(verdicts)
}

/// One subject's readiness, against the snapshot the platform is running.
pub(crate) fn reconcile_subject(
    hook: &HookGlobalId,
    definitions: &dyn HookDefinitionRepository,
    active: &dyn ActiveExtensionSnapshotPort,
) -> Result<SubjectVerdict, String> {
    let active = active.active_snapshot(hook)?;

    // Looked up only for a running snapshot, and only for that snapshot. Reaching for any other
    // revision here is the defect this function exists to remove.
    let recorded_at_active = match &active {
        ActiveSnapshot::Running { snapshot, .. } => definitions
            .recorded(hook, snapshot)?
            .map(|revision| revision.digest),
        _ => None,
    };

    // Only distinguishes "uninstalled, with evidence of what it was" from "a subject that exists
    // because something mentions it". Never used to pick a revision.
    let has_any_revision = !definitions.revisions(hook)?.is_empty();

    Ok(judge_subject(&SubjectFacts {
        hook: hook.clone(),
        active,
        recorded_at_active,
        has_any_revision,
    }))
}

/// Every revision recorded for a subject, most recently recorded first.
///
/// **Diagnostic only.** This is the ordering that must not decide readiness: a recorded-but-not-
/// activated version leads it, and after a rollback the abandoned newer version still leads it.
/// Kept because "what does this installation know about this Hook" is a real question for an
/// operator looking at a subject that will not run — it is just not the question `reconcile_subject`
/// answers.
pub(crate) fn recorded_revisions(
    hook: &HookGlobalId,
    definitions: &dyn HookDefinitionRepository,
) -> Result<Vec<HookDefinitionRevision>, String> {
    definitions.revisions(hook)
}
