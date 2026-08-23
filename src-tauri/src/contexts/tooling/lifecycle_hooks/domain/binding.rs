// The dispatch engine that reads these lands with Task Group 7; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Whether a Hook runs, per scope — the one piece of Hook state a user owns.
//!
//! A binding attaches to the *subject*, not to a definition revision, so it survives upgrade,
//! rollback, and a definition that is momentarily unavailable. Nothing in the install path may
//! write one: seeding a built-in Hook creates its subject and its definition, and if a binding
//! already exists it is left exactly as it is. A seed that "restores the default" is a seed that
//! silently re-enables something a user turned off, and the user finds out when it runs.
//!
//! Moves are compare-and-swap on `revision`. A binding is read, shown, and written back by a
//! human-paced flow, so the window between read and write is wide open; last-write-wins across
//! two windows means one of them is discarded with no one told.

use super::{HookGlobalId, HookScope};

/// Whether a Hook runs in one scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HookBinding {
    pub(crate) hook: HookGlobalId,
    pub(crate) scope: HookScope,
    pub(crate) enabled: bool,
    pub(crate) revision: i64,
    pub(crate) updated_at: String,
}

/// Why a binding could not be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HookBindingError {
    /// Someone else moved the binding since the caller read it.
    ///
    /// Carries both revisions so a caller can say what it expected rather than only that it was
    /// wrong.
    StaleRevision {
        expected: i64,
        actual: i64,
    },
    /// No subject with this id. Refused by the database's reference; reported here so a caller
    /// does not have to read a foreign-key message to find out.
    UnknownSubject,
    Storage(String),
}

impl HookBindingError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::StaleRevision { .. } => "hook_binding_stale_revision",
            Self::UnknownSubject => "unknown_hook_subject",
            Self::Storage(_) => "hook_binding_storage_failure",
        }
    }
}

pub(crate) fn all_hook_binding_errors() -> Vec<HookBindingError> {
    vec![
        HookBindingError::StaleRevision {
            expected: 0,
            actual: 0,
        },
        HookBindingError::UnknownSubject,
        HookBindingError::Storage(String::new()),
    ]
}

/// What a seed is allowed to do to a binding that already exists: nothing.
///
/// Modelled as a value rather than left implicit in a repository method, so the guarantee is
/// something a test can assert about the decision instead of about a SQL statement's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SeedOutcome {
    /// No binding existed. The default applies.
    Seeded,
    /// A binding exists. It is left exactly as it is, whatever the default says.
    Preserved,
}

impl SeedOutcome {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Seeded => "hook_binding_seeded",
            Self::Preserved => "hook_binding_preserved",
        }
    }
}

/// Decides what seeding a default means against whatever the user already has.
pub(crate) fn decide_seed(existing: Option<&HookBinding>) -> SeedOutcome {
    match existing {
        Some(_) => SeedOutcome::Preserved,
        None => SeedOutcome::Seeded,
    }
}
