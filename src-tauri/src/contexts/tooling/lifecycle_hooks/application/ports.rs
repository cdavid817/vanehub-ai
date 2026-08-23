// The services that drive these land with Task Group 7; Task Group 3 lands the adapters.
#![cfg_attr(not(test), allow(dead_code))]

//! What Hook storage is allowed to be asked to do.
//!
//! Four repositories rather than one, because the four aggregates have genuinely different
//! mutability: a subject is an immutable identity, a definition revision is immutable content, a
//! binding is user state under compare-and-swap, and an execution is an append-only row inside a
//! retention window. One repository with all of it would have to be trusted not to offer, say, an
//! `update_execution`, and the type that does not exist is the one nobody can call.
//!
//! `SnapshotProjectionPort` is how this subdomain learns anything about a snapshot. It is a read
//! through `extension_platform`'s published API — never a query against its tables — which is what
//! keeps `snapshot_id` an opaque reference here rather than a foreign key that would let one
//! subdomain's deletions reach into another's evidence.

use crate::contexts::tooling::lifecycle_hooks::domain::{
    ActiveSnapshot, DefinitionOutcome, HookBinding, HookBindingError, HookDefinitionRevision,
    HookExecutionError, HookExecutionRecord, HookExecutionRetention, HookGlobalId, HookScope,
    HookSubject, SeedOutcome, SnapshotRef,
};

/// Stable Hook identities.
///
/// `ensure` is the only writer, and it is idempotent: a subject already present is left alone,
/// including its `first_seen_at`, because when a Hook was first seen is evidence and re-seeding is
/// not a new sighting.
pub(crate) trait HookSubjectRepository: Send + Sync {
    fn ensure(&self, subject: &HookSubject) -> Result<(), String>;

    fn get(&self, hook: &HookGlobalId) -> Result<Option<HookSubject>, String>;

    fn all(&self) -> Result<Vec<HookSubject>, String>;
}

/// Immutable `(subject, snapshot)` definitions.
pub(crate) trait HookDefinitionRepository: Send + Sync {
    /// Records a revision, reporting what it meant against whatever already held the pair.
    ///
    /// Never overwrites. A pair bound to a different digest yields `Conflict` and the stored row
    /// is untouched, so a rebuild cannot change what an already-installed snapshot means.
    fn record(&self, revision: &HookDefinitionRevision) -> Result<DefinitionOutcome, String>;

    fn recorded(
        &self,
        hook: &HookGlobalId,
        snapshot: &SnapshotRef,
    ) -> Result<Option<HookDefinitionRevision>, String>;

    /// Every revision recorded for a subject, most recently recorded first.
    fn revisions(&self, hook: &HookGlobalId) -> Result<Vec<HookDefinitionRevision>, String>;
}

/// User enablement, per scope.
pub(crate) trait HookBindingRepository: Send + Sync {
    fn binding(
        &self,
        hook: &HookGlobalId,
        scope: &HookScope,
    ) -> Result<Option<HookBinding>, HookBindingError>;

    /// Moves a binding, refusing the write if someone else moved it since `expected_revision` was
    /// read. `expected_revision` is `0` for a binding that does not exist yet.
    fn set(
        &self,
        hook: &HookGlobalId,
        scope: &HookScope,
        enabled: bool,
        expected_revision: i64,
        at: &str,
    ) -> Result<HookBinding, HookBindingError>;

    /// Applies a default **only** where the user has no binding.
    ///
    /// Separate from `set` on purpose: a seed that could overwrite is a seed that will, on some
    /// upgrade, silently re-enable something a user turned off.
    fn seed_default(
        &self,
        hook: &HookGlobalId,
        scope: &HookScope,
        enabled: bool,
        at: &str,
    ) -> Result<SeedOutcome, HookBindingError>;

    fn bindings(&self, hook: &HookGlobalId) -> Result<Vec<HookBinding>, HookBindingError>;
}

/// Append-only execution evidence inside a bounded window.
///
/// There is no update and no delete-by-id. The only removal is `prune`, which the retention rule
/// bounds, so "an execution row was edited afterwards" is not a thing this interface can express.
pub(crate) trait HookExecutionRepository: Send + Sync {
    /// Appends a row and returns it with the `sequence` storage assigned.
    ///
    /// The caller does not choose the sequence: it is monotonic per subject and only storage can
    /// see the other writers.
    fn append(
        &self,
        record: &HookExecutionRecord,
    ) -> Result<HookExecutionRecord, HookExecutionError>;

    /// Removes terminal rows outside the window. Returns how many went.
    ///
    /// Unfinished rows are never removed regardless of age — see the domain's retention rule.
    fn prune(
        &self,
        hook: &HookGlobalId,
        retention: HookExecutionRetention,
    ) -> Result<usize, HookExecutionError>;

    /// The most recent executions for a subject, newest first.
    fn recent(
        &self,
        hook: &HookGlobalId,
        limit: usize,
    ) -> Result<Vec<HookExecutionRecord>, HookExecutionError>;
}

/// What snapshot the platform is running for a Hook's contribution, and what it declares.
///
/// Consumer-owned: this subdomain declares the interface it needs and an adapter in its own
/// infrastructure satisfies it by calling `extension_platform`'s published API. The alternative --
/// importing the platform's own type or reading its tables -- would make a change to the platform's
/// model a compile error here, or worse, a silent behaviour change.
///
/// The whole surface is one question about one Hook. Anything wider would be this subdomain
/// reading another's model, and every extra field would be one more thing that could be used to
/// reconstruct a readiness answer without going through the authority chain.
pub(crate) trait ActiveExtensionSnapshotPort: Send + Sync {
    fn active_snapshot(&self, hook: &HookGlobalId) -> Result<ActiveSnapshot, String>;
}
