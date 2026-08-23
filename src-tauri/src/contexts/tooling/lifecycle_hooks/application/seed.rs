// The host dispatch points this seeds land with Task Group 7; see the domain's `builtin.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Seeding the host's own Hook contributions, on every launch, without touching a user's answers.
//!
//! Takes the subject and definition repositories and **nothing else**. There is no binding
//! repository in reach, so this cannot create an enablement, and no amount of later editing makes
//! it possible without someone adding a parameter and being asked why.
//!
//! Every launch runs it. Idempotence is therefore not a nicety: a seed that was not idempotent
//! would produce a different database on the second start than the first, and the second start is
//! the one nobody tests by hand.

use super::{HookDefinitionRepository, HookSubjectRepository};
use crate::contexts::tooling::lifecycle_hooks::domain::{
    decide_owner, BuiltinHookDescriptor, DefinitionOutcome, HookDefinitionRevision, HookOrigin,
    HookSeedOutcome, HookSeedRejection, HookSubject,
};

/// What one pass of the seed did, per descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct HookSeedReport {
    pub(crate) seeded: usize,
    pub(crate) already_seeded: usize,
    pub(crate) revisions_added: usize,
}

impl HookSeedReport {
    fn record(&mut self, outcome: HookSeedOutcome) {
        match outcome {
            HookSeedOutcome::Seeded => self.seeded += 1,
            HookSeedOutcome::AlreadySeeded => self.already_seeded += 1,
            HookSeedOutcome::RevisionAdded => self.revisions_added += 1,
        }
    }

    /// Whether this pass changed anything. A repeated launch reports `false`.
    pub(crate) const fn changed_anything(&self) -> bool {
        self.seeded > 0 || self.revisions_added > 0
    }
}

/// Seeds every descriptor in `catalog`.
///
/// Stops at the first rejection and reports it. Descriptors already applied stay applied: each one
/// is its own subject and its own immutable revision, so a partial pass leaves a consistent
/// database rather than a half-written one, and the next launch resumes from where it stopped.
/// Rolling the earlier ones back would mean deleting subjects that other evidence may already
/// reference — which is precisely what `ON DELETE RESTRICT` exists to refuse.
pub(crate) fn seed_builtin_hooks(
    subjects: &dyn HookSubjectRepository,
    definitions: &dyn HookDefinitionRepository,
    catalog: &[BuiltinHookDescriptor],
    at: &str,
) -> Result<HookSeedReport, HookSeedRejection> {
    let mut report = HookSeedReport::default();
    for descriptor in catalog {
        report.record(seed_one(subjects, definitions, descriptor, at)?);
    }
    Ok(report)
}

fn seed_one(
    subjects: &dyn HookSubjectRepository,
    definitions: &dyn HookDefinitionRepository,
    descriptor: &BuiltinHookDescriptor,
    at: &str,
) -> Result<HookSeedOutcome, HookSeedRejection> {
    let storage = |error: String| HookSeedRejection::Storage(error);

    let stored = subjects
        .get(&descriptor.hook)
        .map_err(storage)?
        .map(|subject| subject.origin);
    decide_owner(&descriptor.hook, stored, HookOrigin::Builtin)?;

    // `ensure` is idempotent and leaves `first_seen_at` alone, so a repeated launch does not move
    // when the host first contributed this hook.
    subjects
        .ensure(&HookSubject {
            hook: descriptor.hook.clone(),
            origin: HookOrigin::Builtin,
            first_seen_at: at.to_string(),
        })
        .map_err(storage)?;

    let outcome = definitions
        .record(&HookDefinitionRevision {
            hook: descriptor.hook.clone(),
            snapshot: descriptor.snapshot(),
            event: descriptor.event,
            digest: descriptor.digest.clone(),
            recorded_at: at.to_string(),
        })
        .map_err(storage)?;

    Ok(match outcome {
        // The subject may have existed while this snapshot's definition did not -- that is an
        // upgrade: the host bumped the built-in snapshot and this is the new revision beside the
        // old one.
        DefinitionOutcome::Recorded if stored.is_some() => HookSeedOutcome::RevisionAdded,
        DefinitionOutcome::Recorded => HookSeedOutcome::Seeded,
        DefinitionOutcome::AlreadyRecorded => HookSeedOutcome::AlreadySeeded,
        // The same built-in snapshot claiming two different definitions means one of two builds is
        // wrong about what it shipped. An upgrade is a new snapshot, not a rewritten one.
        DefinitionOutcome::Conflict(_) => {
            return Err(HookSeedRejection::DefinitionConflict {
                hook: descriptor.hook.clone(),
            })
        }
    })
}
