use chrono::{DateTime, Utc};

use super::error::PersonalizationApplicationError;
use super::migrate_legacy_policy::MigratedPolicy;
use super::models::{
    CreateMemoryInput, DeleteMemoryOutcome, ResetCounts, UpdateMemoryPatch,
    WorkspaceIdentityRequest,
};
use crate::contexts::personalization::domain::{
    AgentId, CandidateReviewStatus, LegacySourceId, MemoryCandidate, MemoryId, MemoryPage,
    MemoryQuery, MemoryRecord, MemoryScopeFilter, MemoryStatus, MigrationJournalEntry,
    MigrationState, PatchPolicyResult, PersonalizationLayers, PersonalizationPolicyPatch,
    PersonalizationPolicyRecord, PersonalizationPolicyScope, ReconcileMemoryOutcome,
    ResetMemoryOutcome, ResetMemoryRequest, StorageEntry, WorkspaceIdentity, WorkspaceKey,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// Durable policy rows, one per typed scope key.
///
/// Every method is synchronous. Policy resolution runs inside the generation call chain, which is
/// synchronous here exactly as `AgentMemoryPort`'s is; making this async would force a block_on at
/// the one place that must not stall a turn.
pub(crate) trait PolicyRepository: Send + Sync {
    fn load(
        &self,
        scope: &PersonalizationPolicyScope,
    ) -> Result<Option<PersonalizationPolicyRecord>>;

    /// The four rows that apply to one resolution context, fetched together.
    ///
    /// One call rather than four so a policy save landing between them cannot produce a snapshot
    /// that mixes revisions — the property the immutable snapshot is supposed to guarantee.
    fn load_layers(
        &self,
        agent_id: &AgentId,
        workspace_key: Option<&WorkspaceKey>,
    ) -> Result<PersonalizationLayers>;

    /// Every stored override. Used by the settings overview to show which scopes have opinions,
    /// never by the runtime.
    fn list_all(&self) -> Result<Vec<PersonalizationPolicyRecord>>;

    /// Creates the default global row if the installation has none. Idempotent: an existing row is
    /// left exactly as it is, including its revision, so startup never resets a user's policy.
    fn seed_default_global(&self, now: DateTime<Utc>) -> Result<PersonalizationPolicyRecord>;

    /// Compare-and-swap in one transaction.
    ///
    /// The read, the revision check, and the write have to be atomic here rather than split across
    /// a load in the service and a write in the repository — otherwise two concurrent saves can
    /// both read revision N, both find it current, and both write N+1, which is last-response-wins
    /// wearing an expected-revision costume.
    fn patch(
        &self,
        scope: &PersonalizationPolicyScope,
        expected_revision: Option<u64>,
        patch: PersonalizationPolicyPatch,
        now: DateTime<Utc>,
    ) -> Result<PatchPolicyResult>;

    fn delete(&self, scope: &PersonalizationPolicyScope) -> Result<bool>;
}

/// The authoritative memory store: Markdown files addressed by immutable id.
///
/// Deliberately has no list operation. The design sketches `list_page` here, but answering it from
/// files would mean reading every body to render a list — the N+1 the same design forbids. Listing
/// belongs to `MemoryProjectionPort`; the application service routes queries there and detail reads
/// here, so a caller cannot accidentally take the expensive path.
pub(crate) trait MemoryRepository: Send + Sync {
    fn get(&self, id: &MemoryId) -> Result<Option<MemoryRecord>>;

    /// Create-new semantics. Never replaces an existing file, so a duplicate display name produces
    /// a second independent record instead of overwriting the first.
    fn create(&self, input: CreateMemoryInput, now: DateTime<Utc>) -> Result<MemoryRecord>;

    fn update(
        &self,
        id: &MemoryId,
        expected_revision: u64,
        patch: UpdateMemoryPatch,
        now: DateTime<Utc>,
    ) -> Result<MemoryRecord>;

    /// Removes the authoritative file only. The returned outcome has `deleted_file` set and every
    /// derived flag left false — the coordinating application service is what removes the
    /// projection row, the index line, and the retrieval entry, and only it can report on all
    /// four.
    fn delete(&self, id: &MemoryId, expected_revision: Option<u64>) -> Result<DeleteMemoryOutcome>;
}

/// Complete, unbounded enumeration for migration, reset, repair, and reconciliation.
///
/// Separate from `MemoryRepository` on purpose, and named so that a reviewer can see at a glance
/// that a caller reached for the unbounded surface. Mixing the two is exactly how a 200-file
/// query cap ended up governing a destructive reset.
pub(crate) trait MemoryMaintenanceRepository: Send + Sync {
    /// Every application-owned entry, classified by explicit rule rather than by whether its
    /// frontmatter happened to parse. Malformed files must appear here.
    fn enumerate_owned_entries(&self) -> Result<Vec<StorageEntry>>;

    fn count_for_reset(
        &self,
        scope: &MemoryScopeFilter,
        statuses: &[MemoryStatus],
    ) -> Result<ResetCounts>;

    fn reset(&self, request: &ResetMemoryRequest, now: DateTime<Utc>)
        -> Result<ResetMemoryOutcome>;

    fn reconcile(&self, now: DateTime<Utc>) -> Result<ReconcileMemoryOutcome>;
}

/// The indexed SQLite projection of the authoritative files.
///
/// Derived, never authoritative. Its job is to answer list, filter, and count queries without
/// reading bodies; when it disagrees with the files, the files win and this is rebuilt.
pub(crate) trait MemoryProjectionPort: Send + Sync {
    fn upsert(&self, record: &MemoryRecord, content_hash: &str) -> Result<()>;
    fn remove(&self, id: &MemoryId) -> Result<bool>;
    fn list_page(&self, query: &MemoryQuery) -> Result<MemoryPage>;
    fn count_for_reset(
        &self,
        scope: &MemoryScopeFilter,
        statuses: &[MemoryStatus],
    ) -> Result<ResetCounts>;
    fn projected_ids(&self) -> Result<Vec<MemoryId>>;
    fn clear(&self) -> Result<usize>;
}

/// Pending and reviewed proposals.
///
/// A candidate is never reachable from `MemoryRepository`, so nothing that enumerates active
/// memories can accidentally surface one.
pub(crate) trait CandidateRepository: Send + Sync {
    fn insert(&self, candidate: &MemoryCandidate) -> Result<()>;
    fn get(&self, candidate_id: &MemoryId) -> Result<Option<MemoryCandidate>>;
    fn list_pending(&self, limit: usize) -> Result<Vec<MemoryCandidate>>;
    fn count_pending(&self) -> Result<usize>;
    fn mark_reviewed(
        &self,
        candidate_id: &MemoryId,
        status: CandidateReviewStatus,
        reviewed_at: DateTime<Utc>,
    ) -> Result<()>;

    /// Drops the proposed content of reviewed candidates beyond the retention bound, keeping only
    /// audit metadata. A rejected extraction must not linger as a second copy of the text the user
    /// declined to keep.
    fn prune_reviewed(&self, retain: usize) -> Result<usize>;
}

/// Coordination with the semantic retrieval subsystem.
///
/// Revocation is idempotent because reconciliation retries it: an entry that is already gone is
/// success, not an error, or repair would never converge.
pub(crate) trait RetrievalIndexPort: Send + Sync {
    fn upsert(&self, record: &MemoryRecord) -> Result<()>;
    fn revoke(&self, id: &MemoryId) -> Result<()>;
    fn revoke_all(&self, ids: &[MemoryId]) -> Result<usize>;
    fn indexed_ids(&self) -> Result<Vec<MemoryId>>;
}

/// Derived `MEMORY.md`. Rebuilt from active records; never read as authoritative.
pub(crate) trait DerivedIndexPort: Send + Sync {
    fn rebuild(&self, active: &[MemoryRecord]) -> Result<usize>;
}

/// Turns whatever the caller knows about a workspace into a stable local key.
pub(crate) trait WorkspaceIdentityPort: Send + Sync {
    fn resolve(&self, request: &WorkspaceIdentityRequest) -> Result<Option<WorkspaceIdentity>>;
}

pub(crate) trait MigrationStatePort: Send + Sync {
    fn load(&self) -> Result<MigrationState>;
    fn save(&self, state: &MigrationState) -> Result<()>;
}

/// One-time migration of the legacy `AppSettings` personalization fields.
///
/// Separate from `PolicyRepository` because it spans two tables — the policy rows and the migration
/// marker — and the whole point is that they move together. A marker written without its rows would
/// make the next startup skip a migration that never happened; rows written without their marker
/// would make it run again over data that had already moved.
pub(crate) trait LegacyPolicyMigrationPort: Send + Sync {
    /// Whether policy migration has already completed. Fails closed on an unreadable marker: a
    /// second migration over already-migrated data is worse than a delayed one.
    fn is_complete(&self) -> Result<bool>;

    /// Commits the mapped rows and the marker in one transaction, or leaves the database exactly as
    /// it was. Returns `false` when migration had already completed, so a repeated startup is a
    /// no-op rather than an error.
    fn commit(&self, migrated: &MigratedPolicy, now: DateTime<Utc>) -> Result<bool>;
}

/// The migration journal, which doubles as the legacy-identity alias table.
///
/// Both readers need the same fact — "which v2 record did this legacy source become" — so they
/// share one table rather than two that could disagree. Migration writes it stage by stage; the
/// compatibility bridge reads it to address a memory by the name that used to be its identity,
/// which searching by display name can no longer do now that duplicates are legal.
pub(crate) trait MigrationJournalPort: Send + Sync {
    fn get(&self, legacy_source_id: &LegacySourceId) -> Result<Option<MigrationJournalEntry>>;

    /// Reverse lookup, so a deleted memory's alias can be found and cleaned up.
    fn find_by_memory(&self, memory_id: &MemoryId) -> Result<Vec<MigrationJournalEntry>>;

    /// Inserts or advances one entry. Persisted before the step it authorizes, never after.
    fn upsert(&self, entry: &MigrationJournalEntry, now: DateTime<Utc>) -> Result<()>;

    /// Every entry, for resuming an interrupted run.
    fn list_all(&self) -> Result<Vec<MigrationJournalEntry>>;

    fn remove(&self, legacy_source_id: &LegacySourceId) -> Result<bool>;
}

/// Injected rather than called directly so the domain stays clock-free and every time-dependent
/// rule is testable without sleeping.
pub(crate) trait ClockPort: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// Allocates immutable memory and candidate ids. A port rather than a helper so a test can produce
/// a deterministic sequence and assert on filenames.
pub(crate) trait MemoryIdGeneratorPort: Send + Sync {
    fn generate(&self) -> MemoryId;
}
