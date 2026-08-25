use chrono::{DateTime, Utc};

use super::error::PersonalizationApplicationError;
use super::migrate_legacy_policy::{LegacyPersonalizationSettings, MigratedPolicy};
use super::models::{
    CreateMemoryInput, DeleteMemoryOutcome, DiscoveredLegacySource, MemoryEligibilityCriteria,
    ResetCounts, UpdateMemoryPatch, WorkspaceIdentityRequest,
};
use crate::contexts::personalization::domain::{
    AgentId, CandidateReviewStatus, LegacyAddressKey, LegacySourceId, LegacySourceLocator,
    MemoryCandidate, MemoryEligibilitySummary, MemoryId, MemoryPage, MemoryQuery, MemoryRecord,
    MemoryRuntimeHealth, MemoryScopeFilter, MemoryStatus, MigrationJournalEntry, MigrationState,
    PatchPolicyResult, PersonalizationLayers, PersonalizationPolicyPatch,
    PersonalizationPolicyRecord, PersonalizationPolicyScope, PersonalizationRuntimeCapabilities,
    PolicyResolutionBundle, ReconcileMemoryOutcome, ResetMemoryOutcome, ResetMemoryRequest,
    StorageEntry, WorkspaceIdentity, WorkspaceKey,
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

    /// Every scope key one resolution needs, read together, with each key's finding.
    ///
    /// One consistent read rather than four: a save landing between two of them would produce a
    /// snapshot mixing revisions, which is exactly what an immutable snapshot is supposed to rule
    /// out. Returns `Absent` for a key with no override rather than omitting it, so a caller can
    /// tell "proved there is none" from "never looked" — the distinction that decides whether the
    /// result may be reused later.
    fn load_resolution_bundle(
        &self,
        scopes: &[PersonalizationPolicyScope],
    ) -> Result<PolicyResolutionBundle>;

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

    /// Creates at a caller-chosen id. Migration only.
    ///
    /// Ordinary creation deliberately gives no way to propose an id, so no caller can reuse one.
    /// Migration is the exception because its id has to be journalled *before* the file is written:
    /// if the id were allocated at write time, a crash between the write and the journal entry
    /// would leave an orphan file that the next run could not recognize, and it would create a
    /// second record for the same source. Create-new semantics still apply, so a resumed run that
    /// finds its own earlier file gets an error rather than silently overwriting it.
    ///
    /// Takes both timestamps because a migrated record has two real ones: the creation time its
    /// source declared, and the modification time the filesystem knows. Ordinary creation collapses
    /// them because for a record being created now they genuinely are the same instant.
    fn create_with_id(
        &self,
        id: &MemoryId,
        input: CreateMemoryInput,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<MemoryRecord>;

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
    /// Which memories are eligible right now, bounded, with exact counts.
    ///
    /// Answered here rather than by reading files because it runs before token budgeting and
    /// relevance selection: loading every body to decide what fits would defeat the budgeting it
    /// feeds. Returns refs without bodies, the exact eligible total, and one primary exclusion
    /// reason per excluded record — so `eligible_total + sum(exclusions) == considered` holds by
    /// construction, and a user reading "3 of 40" can see where the other 37 went.
    fn eligible_page(
        &self,
        criteria: &MemoryEligibilityCriteria,
    ) -> Result<MemoryEligibilitySummary>;

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

/// What an Agent's runtime adapter declares it can consume.
///
/// A port because the registry belongs to another context, and personalization must not reach into
/// it. `None` means the registry has no such Agent — reported rather than defaulted, because a
/// default capability set for an Agent nobody registered would grant a surface nothing declared.
///
/// Deliberately keyed by id alone. Nothing here enumerates Agents or branches on which one it is,
/// so an Agent registered at runtime resolves through exactly the same path as a built-in.
pub(crate) trait AgentCapabilityPort: Send + Sync {
    fn capabilities(
        &self,
        agent_id: &AgentId,
    ) -> Result<Option<PersonalizationRuntimeCapabilities>>;
}

/// Removes anything secret-looking from text before it is shown or recorded.
///
/// A port because the rule belongs to the platform and this layer must not reach into it, and
/// because the preview and the logs have to apply the *same* rule: a screen that redacted less than
/// a log file would be the place a token escaped.
pub(crate) trait SecretRedactionPort: Send + Sync {
    fn redact(&self, text: &str) -> String;
}

/// Turns whatever the caller knows about a workspace into a stable local key.
pub(crate) trait WorkspaceIdentityPort: Send + Sync {
    fn resolve(&self, request: &WorkspaceIdentityRequest) -> Result<Option<WorkspaceIdentity>>;
}

/// Whether stored memory may be used right now.
///
/// A port rather than a direct read of the durable row because the answer is not the row alone: a
/// process that found maintenance held by another one knows something the row does not say. One
/// port means every runtime path asks the same question of the same authority.
pub(crate) trait MemoryHealthPort: Send + Sync {
    fn health(&self) -> MemoryRuntimeHealth;
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

/// Decides who owns one memory directory right now: an ordinary mutation, or maintenance.
///
/// The two are mutually exclusive, and that exclusion is this gate rather than a health check.
/// Reading `MemoryRuntimeHealth` and then taking the directory lock leaves a window: a writer that
/// read `Ready` and resumed after maintenance began would still get the directory lock, because
/// maintenance releases it between each of its own operations and holds none at all during the
/// derived rebuild. It would then mutate underneath a reconciliation that is about to rebuild the
/// projection and the index from a snapshot taken before that write.
///
/// So an ordinary mutation holds a shared admission for the whole operation — health check
/// included — and maintenance holds the gate exclusively for its whole run.
pub(crate) trait MaintenanceGatePort: Send + Sync {
    /// Admits one ordinary mutation, or reports that maintenance owns the directory.
    ///
    /// Re-entrant on one thread: a coordinated write takes this around the whole operation and the
    /// store takes it again inside, and maintenance's own writes take it while maintenance holds
    /// the gate. A non-re-entrant lock would report each of those busy.
    fn enter_mutation(&self) -> Result<Box<dyn MutationAdmission>>;

    /// Takes the gate exclusively, or reports that someone else already has it. Never blocks: a
    /// second process must report busy and let unrelated features start, not park a thread until
    /// the first finishes.
    fn try_enter_maintenance(&self) -> Result<Option<Box<dyn MaintenanceLease>>>;
}

/// One ordinary mutation's claim, released on drop.
pub(crate) trait MutationAdmission: Send {}

/// Held for as long as maintenance runs, and released on drop — including on panic, and by the
/// operating system if the process dies outright, so a crash leaves nothing stale behind.
pub(crate) trait MaintenanceLease: Send {}

/// The frozen conversion from the pre-file row store into v1 files.
///
/// A port rather than a direct call because the row store belongs to another context. Frozen
/// because it gains no behaviour: it exists so the rows and the files are converted by one
/// orchestration in one order, instead of two startup paths racing over one directory.
pub(crate) trait LegacyRowMigrationPort: Send + Sync {
    /// Converts every pre-file row into a v1 file, and reports how many were written.
    ///
    /// Idempotence is the orchestration's, not this port's: it is called once, gated by a durable
    /// marker, because re-running it after v2 records exist would resurrect memories the user has
    /// since deleted.
    fn convert_rows_to_legacy_files(&self) -> Result<usize>;
}

/// The pre-governance settings, read once so their personalization fields can be migrated.
///
/// Read-only on purpose. After migration the dedicated policy is the source of truth, and a write
/// back through here would recreate the second truth this change exists to end.
pub(crate) trait LegacyPersonalizationSettingsPort: Send + Sync {
    fn load(&self) -> Result<LegacyPersonalizationSettings>;
}

/// The pre-v2 store, read for migration only.
///
/// Separate from every other port because its lifetime is the migration's: once no legacy source
/// remains, nothing calls it. Keeping it apart is what makes "is anything still reading v1" a
/// question with a grep-able answer.
pub(crate) trait LegacyMemorySourcePort: Send + Sync {
    /// Every application-owned legacy source, with its raw fingerprint and parsed fields.
    ///
    /// Complete and uncapped. Excludes the derived index, the lock file, temporaries, the backup
    /// and quarantine directories, and anything already in v2 format — the last decided by reading
    /// the declared schema version, never by the shape of the filename.
    fn enumerate_sources(&self) -> Result<Vec<DiscoveredLegacySource>>;

    /// Raw bytes, for fingerprinting and for the backup. Never the parsed body: a backup has to
    /// restore the file an older build could read, which means the original bytes.
    fn read_raw(&self, locator: &LegacySourceLocator) -> Result<Vec<u8>>;

    /// Writes a byte-for-byte copy into the backup directory and returns its relative path.
    fn write_backup(&self, locator: &LegacySourceLocator, bytes: &[u8]) -> Result<String>;

    /// Re-reads a backup so its bytes can be checked against the source's fingerprint.
    fn read_backup(&self, relative_path: &str) -> Result<Vec<u8>>;

    /// Removes a legacy source. Only ever called after its backup and its v2 record are verified.
    fn remove_source(&self, locator: &LegacySourceLocator) -> Result<()>;

    /// Moves an unusable source into quarantine, returning its new relative path. Quarantine and
    /// backup are different directories with different meanings: a backup exists so a good source
    /// can be restored, a quarantine exists so a bad one is not lost.
    fn quarantine_source(&self, locator: &LegacySourceLocator) -> Result<String>;
}

/// Compatibility addressing: which v2 record an old display-name-derived address points at.
///
/// Typed on `LegacyAddressKey` so a source id cannot be passed here. The two identities answer
/// different questions and there is deliberately no conversion between them.
pub(crate) trait LegacyAddressAliasPort: Send + Sync {
    fn get(&self, address: &LegacyAddressKey) -> Result<Option<MemoryId>>;
    fn put(&self, address: &LegacyAddressKey, target: &MemoryId, now: DateTime<Utc>) -> Result<()>;
    fn remove(&self, address: &LegacyAddressKey) -> Result<bool>;
    fn list_all(&self) -> Result<Vec<(LegacyAddressKey, MemoryId)>>;
}

/// The migration journal: which stage each actually-discovered source has reached.
///
/// Typed on `LegacySourceId`, which comes from where the source was found. Keying this on a display
/// name would make two same-named files one journal entry, and a malformed file — which has no
/// readable name at all — unjournalable.
pub(crate) trait MigrationJournalPort: Send + Sync {
    fn get(&self, source_id: &LegacySourceId) -> Result<Option<MigrationJournalEntry>>;

    /// Reverse lookup, so a target's journal history can be found from the record.
    fn find_by_memory(&self, memory_id: &MemoryId) -> Result<Vec<MigrationJournalEntry>>;

    /// Inserts one entry only if this source has none, and returns whichever entry is stored.
    ///
    /// Atomic, and the reason two migrators over one directory converge instead of duplicating: the
    /// target memory id is chosen here, before anything is written, so the loser of the race adopts
    /// the winner's id rather than allocating a second one and producing a second record for the
    /// same source.
    fn claim(
        &self,
        entry: &MigrationJournalEntry,
        now: DateTime<Utc>,
    ) -> Result<MigrationJournalEntry>;

    /// Inserts or advances one entry. Persisted before the step it authorizes, never after.
    fn upsert(&self, entry: &MigrationJournalEntry, now: DateTime<Utc>) -> Result<()>;

    /// Every entry, for resuming an interrupted run.
    fn list_all(&self) -> Result<Vec<MigrationJournalEntry>>;

    fn remove(&self, source_id: &LegacySourceId) -> Result<bool>;
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
