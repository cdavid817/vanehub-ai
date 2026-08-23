// The host dispatch points these describe land with Task Group 7; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! What the host itself contributes, and what seeding it may and may not do.
//!
//! ## Seeding creates identity, never user state
//!
//! A seed may create a subject and a definition revision. It may not create a binding, an
//! enablement, a configuration, or a trust decision — those are answers a person gave, and a
//! process that runs on every launch has no business producing them. The mechanism enforces this
//! by not having a binding repository in reach: `seed_builtin_hooks` takes the subject and
//! definition repositories and nothing else.
//!
//! ## Ownership is checked, not assumed
//!
//! A subject id is claimed by exactly one owner. If `vanehub.session-start` is already recorded as
//! an *extension's* subject, the seed must not quietly take it over — that would let a launch
//! silently reassign a contribution an operator installed. And an extension must not be able to
//! claim a built-in id either. Both directions are `OwnerConflict`, reported and refused.
//!
//! There is no `INSERT OR REPLACE` anywhere in this path. Replace is exactly the operation that
//! turns both of those conflicts into silent overwrites.
//!
//! ## Upgrades are new revisions, not edits
//!
//! When a built-in definition changes, the host writes a **new immutable revision** under a new
//! built-in snapshot. The old revision stays, which is what lets an operator see that it changed
//! and what makes a downgrade describable. Editing the existing row in place would leave no record
//! that yesterday's build ran something else.

use super::{DefinitionDigest, HookEvent, HookGlobalId, HookOrigin, SnapshotRef};

/// The snapshot built-in definitions are recorded under.
///
/// Built-ins do not come from an extension package, so they have no extension snapshot. A reserved
/// one keeps the `(subject, snapshot)` key uniform — the alternative, a nullable snapshot, would
/// make the primary key partial and reintroduce SQLite's NULL-uniqueness trap.
///
/// It carries a generation suffix because an upgrade is a new revision: the host bumps this when a
/// built-in definition changes, and the previous revision stays where it is.
pub(crate) const BUILTIN_HOOK_SNAPSHOT: &str = "builtin-1";

/// One thing the host contributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BuiltinHookDescriptor {
    pub(crate) hook: HookGlobalId,
    pub(crate) event: HookEvent,
    pub(crate) digest: DefinitionDigest,
}

impl BuiltinHookDescriptor {
    pub(crate) fn snapshot(&self) -> SnapshotRef {
        SnapshotRef::parse(BUILTIN_HOOK_SNAPSHOT).unwrap_or_else(|_| unreachable!())
    }
}

/// What the host contributes in this build.
///
/// Empty today. The host's own dispatch points are defined by Task Group 7, and seeding
/// descriptors for hooks nothing dispatches would create rows no code reads and no test can
/// meaningfully exercise. The mechanism below is complete and tested against catalogs supplied by
/// its callers; what is missing is content, not machinery.
pub(crate) fn builtin_hook_catalog() -> Vec<BuiltinHookDescriptor> {
    Vec::new()
}

/// Why a seed could not proceed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HookSeedRejection {
    /// The subject id is already claimed by a different owner.
    OwnerConflict {
        hook: HookGlobalId,
        stored: HookOrigin,
        offered: HookOrigin,
    },
    /// The same `(subject, snapshot)` is recorded with a different definition.
    ///
    /// Not an upgrade: an upgrade is a *new* snapshot. This is the same snapshot claiming two
    /// different definitions, which means one of the two builds is wrong about what it shipped.
    DefinitionConflict {
        hook: HookGlobalId,
    },
    Storage(String),
}

impl HookSeedRejection {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::OwnerConflict { .. } => "builtin_hook_owner_conflict",
            Self::DefinitionConflict { .. } => "builtin_hook_definition_conflict",
            Self::Storage(_) => "builtin_hook_seed_storage_failure",
        }
    }
}

pub(crate) fn all_hook_seed_rejections() -> Vec<HookSeedRejection> {
    vec![
        HookSeedRejection::OwnerConflict {
            hook: HookGlobalId::parse("placeholder").unwrap_or_else(|_| unreachable!()),
            stored: HookOrigin::Extension,
            offered: HookOrigin::Builtin,
        },
        HookSeedRejection::DefinitionConflict {
            hook: HookGlobalId::parse("placeholder").unwrap_or_else(|_| unreachable!()),
        },
        HookSeedRejection::Storage(String::new()),
    ]
}

/// What seeding one descriptor did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HookSeedOutcome {
    /// The subject and its definition were created.
    Seeded,
    /// Everything was already there, under the same owner and the same digest.
    AlreadySeeded,
    /// The subject was there; this build's definition is a new revision beside the old one.
    RevisionAdded,
}

impl HookSeedOutcome {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Seeded => "builtin_hook_seeded",
            Self::AlreadySeeded => "builtin_hook_already_seeded",
            Self::RevisionAdded => "builtin_hook_revision_added",
        }
    }
}

/// Whether an existing subject may be seeded under `offered`.
///
/// Pure, so the rule is one place. Same owner is fine — that is the ordinary repeated launch.
/// Different owner is refused in both directions: a seed taking over an extension's id would
/// reassign a contribution an operator installed, and an extension taking a built-in id would let
/// a package impersonate the host.
pub(crate) fn decide_owner(
    hook: &HookGlobalId,
    stored: Option<HookOrigin>,
    offered: HookOrigin,
) -> Result<(), HookSeedRejection> {
    match stored {
        None => Ok(()),
        Some(stored) if stored == offered => Ok(()),
        Some(stored) => Err(HookSeedRejection::OwnerConflict {
            hook: hook.clone(),
            stored,
            offered,
        }),
    }
}
