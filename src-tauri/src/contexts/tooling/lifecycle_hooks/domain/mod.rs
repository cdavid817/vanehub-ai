//! Lifecycle Hook subjects, versioned definitions, user bindings, and execution evidence.
//!
//! The rules here are storage-shaped on purpose: Task Group 3 lands what a Hook *is* and what may
//! be written about one, and Task Group 7 lands the engine that dispatches them. Splitting it the
//! other way would have meant an engine deciding its own storage contract, which is how a payload
//! ends up in a durable row.

mod binding;
#[cfg(test)]
mod binding_tests;
mod definition;
#[cfg(test)]
mod definition_tests;
mod execution;
#[cfg(test)]
mod execution_tests;
mod identity;
#[cfg(test)]
mod identity_tests;
mod reconciliation;
#[cfg(test)]
mod reconciliation_tests;

#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use binding::{
    all_hook_binding_errors, decide_seed, HookBinding, HookBindingError, SeedOutcome,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use definition::{
    decide_definition, DefinitionContentConflict, DefinitionOutcome, HookDefinitionRevision,
    HookEvent, HookOrigin, HookSubject, ALL_HOOK_EVENTS,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use execution::{
    all_hook_execution_errors, HookExecutionError, HookExecutionRecord, HookExecutionRetention,
    HookExecutionStatus, ALL_HOOK_EXECUTION_STATUSES, DEFAULT_HOOK_EXECUTION_RETENTION,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use identity::{
    DefinitionDigest, HookExecutionId, HookGlobalId, HookIdentityError, HookOutcomeCode, HookScope,
    HookScopeKind, SnapshotRef, ALL_HOOK_SCOPE_KINDS, GLOBAL_SCOPE_KEY,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use reconciliation::{
    judge_subject, SnapshotFact, SubjectProjection, SubjectReadiness, SubjectVerdict,
    ALL_SUBJECT_READINESS,
};

/// Which identity failed to parse.
///
/// One enum rather than one error type per newtype, so that every rejection presents a stable
/// code from a single list and two of them cannot drift into saying the same thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum HookIdentifierKind {
    HookGlobal,
    SnapshotRef,
    HookExecution,
    DefinitionDigest,
    OutcomeCode,
    ScopeKind,
    ScopeKey,
}

impl HookIdentifierKind {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::HookGlobal => "invalid_hook_global_id",
            Self::SnapshotRef => "invalid_hook_snapshot_ref",
            Self::HookExecution => "invalid_hook_execution_id",
            Self::DefinitionDigest => "invalid_hook_definition_digest",
            Self::OutcomeCode => "invalid_hook_outcome_code",
            Self::ScopeKind => "invalid_hook_scope_kind",
            Self::ScopeKey => "invalid_hook_scope_key",
        }
    }
}

// Read by the catalog test that keeps this list from drifting away from the enum; the report that
// enumerates failure codes for the frontend lands with Task Group 7.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const ALL_HOOK_IDENTIFIER_KINDS: &[HookIdentifierKind] = &[
    HookIdentifierKind::HookGlobal,
    HookIdentifierKind::SnapshotRef,
    HookIdentifierKind::HookExecution,
    HookIdentifierKind::DefinitionDigest,
    HookIdentifierKind::OutcomeCode,
    HookIdentifierKind::ScopeKind,
    HookIdentifierKind::ScopeKey,
];

/// Every stable failure code this subdomain can present to a caller.
///
/// The same invariant `extension_platform` keeps for its own catalog, kept separately because the
/// two subdomains must not have to agree on a shared list to stay distinct from each other. What
/// matters is that no two failures *within* one subdomain collide, since that is the set a caller
/// branches over.
#[cfg(test)]
pub(crate) fn registered_hook_failures() -> Vec<&'static str> {
    let mut codes: Vec<&'static str> = ALL_HOOK_IDENTIFIER_KINDS
        .iter()
        .map(|kind| kind.code())
        .collect();
    codes.extend(
        all_hook_binding_errors()
            .iter()
            .map(HookBindingError::code)
            .collect::<Vec<_>>(),
    );
    codes.extend(
        all_hook_execution_errors()
            .iter()
            .map(HookExecutionError::code)
            .collect::<Vec<_>>(),
    );
    codes.push(DefinitionOutcome::Recorded.code());
    codes.push(DefinitionOutcome::AlreadyRecorded.code());
    codes.push("hook_definition_content_conflict");
    codes.push(SeedOutcome::Seeded.code());
    codes.push(SeedOutcome::Preserved.code());
    codes
}
