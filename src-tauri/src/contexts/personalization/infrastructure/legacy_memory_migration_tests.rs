//! Recoverable migration of the pre-v2 memory store.
//!
//! Run against the real directory, the real files, and the real SQLite journal rather than fakes.
//! Every property here is about what survives an interruption, and a fake store would only restate
//! the ordering it was built from instead of proving the files on disk end up in that order.
//!
//! Lives beside the adapters rather than beside the service it exercises: assembling the concrete
//! store, source, and journal is an adapter-side act, and an application-layer file that reached for
//! them would point its dependencies outward.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use tempfile::TempDir;

use super::{
    FileLegacyMemorySource, LegacyOperation, MarkdownMemoryRepository, MemoryDirectoryLock,
    SqliteLegacyAddressAlias, SqliteMemoryProjection, SqliteMigrationJournal,
    UuidMemoryIdGenerator,
};
use crate::contexts::personalization::application::{
    ClockPort, CreateMemoryInput, LegacyAddressAliasPort, LegacyMemoryMigrationPorts,
    LegacyMemoryMigrationService, LegacyMemorySourcePort, MemoryMaintenanceRepository,
    MemoryProjectionPort, MemoryRepository, MigrationJournalPort, MigrationRunOutcome,
    PersonalizationApplicationError, ResetCounts, WorkspaceIdentityResolver,
};
use crate::contexts::personalization::domain::{
    LegacyAddressKey, LegacySourceLocator, MemoryAudience, MemoryId, MemoryPage, MemoryProvenance,
    MemoryQuery, MemoryRecord, MemoryScope, MemoryScopeFilter, MemorySensitivity, MemorySource,
    MemoryStatus, MemoryType, MigrationJournalEntry, MigrationStage, OwnedEntryClassification,
};
use crate::platform::database::NativeDatabase;

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// Every journal boundary a run can be interrupted at.
///
/// One source per point rather than a cross product with the scale cases: the property is that each
/// boundary is individually recoverable, and multiplying it by a thousand files would test the
/// filesystem's throughput instead.
const INTERRUPT_POINTS: &[MigrationStage] = &[
    MigrationStage::Discovered,
    MigrationStage::BackupWritten,
    MigrationStage::BackupVerified,
    MigrationStage::V2Written,
    MigrationStage::V2Verified,
    MigrationStage::ProjectionWritten,
    MigrationStage::LegacyRemoved,
    MigrationStage::DerivedPending,
];

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 24, 9, 0, 0).unwrap()
}

struct FixedClock;

impl ClockPort for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        now()
    }
}

/// The real projection with a switch that makes its writes fail.
///
/// A wrapper rather than a fake so the success path exercises the actual SQLite statements: the
/// failure being simulated is a disk or lock error, not a different projection.
struct SwitchableProjection {
    inner: SqliteMemoryProjection,
    fails: AtomicBool,
}

impl SwitchableProjection {
    fn new(database: NativeDatabase) -> Self {
        Self {
            inner: SqliteMemoryProjection::new(database),
            fails: AtomicBool::new(false),
        }
    }

    fn set_failing(&self, failing: bool) {
        self.fails.store(failing, Ordering::SeqCst);
    }
}

impl MemoryProjectionPort for SwitchableProjection {
    fn upsert(&self, record: &MemoryRecord, content_hash: &str) -> Result<()> {
        if self.fails.load(Ordering::SeqCst) {
            return Err(PersonalizationApplicationError::Storage(
                "projection_injected_failure".to_string(),
            ));
        }
        self.inner.upsert(record, content_hash)
    }

    fn remove(&self, id: &MemoryId) -> Result<bool> {
        self.inner.remove(id)
    }

    fn list_page(&self, query: &MemoryQuery) -> Result<MemoryPage> {
        self.inner.list_page(query)
    }

    fn count_for_reset(
        &self,
        scope: &MemoryScopeFilter,
        statuses: &[MemoryStatus],
    ) -> Result<ResetCounts> {
        self.inner.count_for_reset(scope, statuses)
    }

    fn projected_ids(&self) -> Result<Vec<MemoryId>> {
        self.inner.projected_ids()
    }

    fn clear(&self) -> Result<usize> {
        self.inner.clear()
    }
}

struct Fixture {
    /// `None` for a stack rebuilt over an existing directory: the original still owns it, and a
    /// second owner would delete it out from under the first when it dropped.
    _directory: Option<TempDir>,
    directory_path: std::path::PathBuf,
    root: std::path::PathBuf,
    sources: Arc<FileLegacyMemorySource>,
    repository: Arc<MarkdownMemoryRepository>,
    projection: Arc<SwitchableProjection>,
    journal: Arc<SqliteMigrationJournal>,
    aliases: Arc<SqliteLegacyAddressAlias>,
    lock: Arc<MemoryDirectoryLock>,
    service: LegacyMemoryMigrationService,
}

fn fixture(label: &str) -> Fixture {
    let directory =
        TempDir::with_prefix(format!("personalization-migrate-{label}-")).expect("temporary dir");
    let path = directory.path().to_path_buf();
    reopen(path, Some(directory))
}

/// Rebuilds the whole stack over an existing directory, standing in for an application restart.
fn reopen(directory_path: std::path::PathBuf, keep: Option<TempDir>) -> Fixture {
    let database = NativeDatabase::new(directory_path.clone()).expect("database");
    let root = directory_path.join("memory");
    let repository = Arc::new(
        MarkdownMemoryRepository::new(root.clone(), Arc::new(UuidMemoryIdGenerator))
            .expect("repository"),
    );
    let lock = repository.lock();
    let sources =
        Arc::new(FileLegacyMemorySource::new(root.clone(), lock.clone()).expect("legacy source"));
    let projection = Arc::new(SwitchableProjection::new(database.clone()));
    let journal = Arc::new(SqliteMigrationJournal::new(database.clone()));
    let aliases = Arc::new(SqliteLegacyAddressAlias::new(database));
    let service = LegacyMemoryMigrationService::new(LegacyMemoryMigrationPorts {
        sources: sources.clone(),
        repository: repository.clone(),
        projection: projection.clone(),
        journal: journal.clone(),
        aliases: aliases.clone(),
        identity: Arc::new(WorkspaceIdentityResolver::for_this_platform()),
        ids: Arc::new(UuidMemoryIdGenerator),
        clock: Arc::new(FixedClock),
    });
    Fixture {
        _directory: keep,
        directory_path,
        root,
        sources,
        repository,
        projection,
        journal,
        aliases,
        lock,
        service,
    }
}

/// A v1 file with the frontmatter v1 actually wrote.
fn legacy_file(name: &str, body: &str) -> String {
    format!(
        "---\nname: {name}\ndescription: About {name}\ntype: project\nagent: onepiece\nfolder: D:/code/vanehub-ai\nsource: explicit\ncreated: 2026-08-01T10:00:00.000Z\n---\n\n{body}\n"
    )
}

impl Fixture {
    fn write_legacy(&self, file_name: &str, contents: &str) {
        std::fs::write(self.root.join(file_name), contents).expect("legacy fixture");
    }

    fn seed(&self, name: &str, body: &str) {
        self.write_legacy(&format!("{name}.md"), &legacy_file(name, body));
    }

    fn run(&self) -> MigrationRunOutcome {
        self.service.run().expect("migration run")
    }

    fn entry(&self, file_name: &str) -> MigrationJournalEntry {
        let locator = LegacySourceLocator::markdown(file_name).expect("locator");
        self.journal
            .get(&locator.source_id())
            .expect("journal read")
            .unwrap_or_else(|| panic!("no journal entry for {file_name}"))
    }

    /// Every v2 record in the directory, read back through the store.
    fn records(&self) -> Vec<MemoryRecord> {
        let mut records = Vec::new();
        for entry in self
            .repository
            .enumerate_owned_entries()
            .expect("enumerate v2")
        {
            if entry.classification != OwnedEntryClassification::ValidV2 {
                continue;
            }
            let id = entry.memory_id.expect("a valid v2 entry has an id");
            records.push(self.repository.get(&id).expect("read").expect("present"));
        }
        records.sort_by(|left, right| left.name.cmp(&right.name));
        records
    }

    fn legacy_exists(&self, file_name: &str) -> bool {
        self.root.join(file_name).exists()
    }

    fn backup_bytes(&self, file_name: &str) -> Vec<u8> {
        std::fs::read(self.root.join("legacy-backup").join(file_name)).expect("backup")
    }

    fn restart(self) -> Fixture {
        let path = self.directory_path.clone();
        let keep = self._directory;
        reopen(path, keep)
    }
}

#[test]
fn an_empty_directory_migrates_nothing_and_reports_nothing() {
    let fixture = fixture("empty");

    let outcome = fixture.run();

    assert_eq!(outcome, MigrationRunOutcome::default());
    assert!(!outcome.requires_repair());
}

#[test]
fn one_legacy_memory_becomes_one_governed_record() {
    let fixture = fixture("single");
    fixture.seed("user-role", "Prefers concise answers.");
    let original = std::fs::read(fixture.root.join("user-role.md")).expect("original bytes");

    let outcome = fixture.run();

    assert_eq!(outcome.discovered, 1);
    assert_eq!(outcome.migrated, 1);
    assert!(!outcome.requires_repair());

    let records = fixture.records();
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.name, "user-role");
    assert_eq!(record.description, "About user-role");
    assert_eq!(record.content, "Prefers concise answers.");
    assert_eq!(record.memory_type, MemoryType::Project);
    // Global scope and an all-Agents audience preserve what a v1 memory could see. Anything
    // narrower would make a memory the user could read yesterday invisible today.
    assert_eq!(record.scope, MemoryScope::Global);
    assert_eq!(record.audience, MemoryAudience::AllAgents);
    assert_eq!(record.status, MemoryStatus::Active);
    assert_eq!(record.source, MemorySource::LegacyMigration);
    assert_eq!(record.revision, 1);
    assert_eq!(
        record
            .provenance
            .source_agent_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("onepiece")
    );
    // The raw v1 folder was a display path, which no workspace key may hold. It is resolved into a
    // stable key rather than dropped.
    assert!(record.provenance.source_workspace_key.is_some());
    assert_eq!(
        record.created_at,
        Utc.with_ymd_and_hms(2026, 8, 1, 10, 0, 0).unwrap()
    );

    // The backup reproduces the original bytes, and only then is the source gone.
    assert_eq!(fixture.backup_bytes("user-role.md"), original);
    assert!(!fixture.legacy_exists("user-role.md"));

    let entry = fixture.entry("user-role.md");
    assert_eq!(entry.stage, MigrationStage::Completed);
    assert_eq!(entry.target_memory_id.as_ref(), Some(&record.id));
    assert_eq!(
        entry.backup_relative_path.as_deref(),
        Some("legacy-backup/user-role.md")
    );

    // The projection can answer for it, and the old display-name address still resolves.
    assert_eq!(
        fixture.projection.projected_ids().expect("projected"),
        vec![record.id.clone()]
    );
    assert_eq!(
        fixture
            .aliases
            .get(&LegacyAddressKey::parse("user-role.md").expect("address"))
            .expect("alias"),
        Some(record.id.clone())
    );
}

#[test]
fn an_unrecognized_legacy_type_migrates_as_explicitly_untyped() {
    // A wrong type is worse than a missing one, and `Untyped` is reachable only for a record whose
    // source is `legacy_migration`, so it cannot leak into anything a user creates.
    let fixture = fixture("unknown-type");
    fixture.write_legacy(
        "odd.md",
        "---\nname: odd\ndescription: d\ntype: something-else\n---\n\nBody.\n",
    );

    fixture.run();

    let records = fixture.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].memory_type, MemoryType::Untyped);
    assert_eq!(records[0].source, MemorySource::LegacyMigration);
}

#[test]
fn a_file_already_in_v2_format_is_not_migrated_again() {
    let fixture = fixture("already-v2");
    let existing = fixture
        .repository
        .create(
            CreateMemoryInput {
                name: "already".to_string(),
                description: "d".to_string(),
                memory_type: MemoryType::Project,
                content: "Body.".to_string(),
                scope: MemoryScope::Global,
                audience: MemoryAudience::AllAgents,
                status: MemoryStatus::Active,
                source: MemorySource::ExplicitUser,
                provenance: MemoryProvenance::default(),
                sensitivity: MemorySensitivity::Normal,
            },
            now(),
        )
        .expect("existing v2 record");

    let outcome = fixture.run();

    assert_eq!(outcome.discovered, 0);
    let records = fixture.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].id, existing.id);
    // Untouched: still explicit_user, still revision 1, still no journal entry.
    assert_eq!(records[0].source, MemorySource::ExplicitUser);
    assert!(fixture.journal.list_all().expect("journal").is_empty());
}

#[test]
fn a_malformed_source_is_quarantined_with_its_bytes_intact() {
    let fixture = fixture("quarantine");
    fixture.seed("good", "Body.");
    fixture.write_legacy("broken.md", "---\nname: broken\ndescription: d\n");

    let outcome = fixture.run();

    assert_eq!(outcome.discovered, 2);
    assert_eq!(outcome.migrated, 1);
    assert_eq!(outcome.quarantined, 1);

    // Moved, never deleted, and never activated.
    assert!(!fixture.legacy_exists("broken.md"));
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("quarantine").join("broken.md"))
            .expect("quarantined"),
        "---\nname: broken\ndescription: d\n"
    );
    let records = fixture.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].name, "good");

    let entry = fixture.entry("broken.md");
    assert_eq!(entry.stage, MigrationStage::Failed);
    assert_eq!(entry.last_error_code.as_deref(), Some("quarantined"));
    assert_eq!(
        entry.backup_relative_path.as_deref(),
        Some("quarantine/broken.md")
    );
}

#[test]
fn a_quarantine_that_cannot_move_the_file_leaves_it_exactly_where_it_is() {
    // Losing a user's text because this build could not move it is worse than any amount of
    // maintenance noise, so a failed quarantine reports and stops rather than deleting.
    let fixture = fixture("quarantine-fails");
    fixture.write_legacy("broken.md", "---\nname: broken\n");
    fixture
        .sources
        .inject_failure(LegacyOperation::Quarantine, "broken.md");

    let outcome = fixture.run();

    assert_eq!(outcome.failed, 1);
    assert!(outcome.requires_repair());
    assert!(fixture.legacy_exists("broken.md"));
    assert_eq!(
        fixture.entry("broken.md").last_error_code.as_deref(),
        Some("quarantine_failed")
    );
}

#[test]
fn two_sources_with_the_same_display_name_become_two_independent_records() {
    // The v1 store keyed on the name, so saving a second memory under an existing name replaced the
    // first. Under v2 they are two records, and only the first claims the old display-name address.
    let fixture = fixture("duplicate-names");
    fixture.write_legacy("first.md", &legacy_file("shared", "First body."));
    fixture.write_legacy("second.md", &legacy_file("shared", "Second body."));

    let outcome = fixture.run();

    assert_eq!(outcome.migrated, 2);
    let records = fixture.records();
    assert_eq!(records.len(), 2);
    assert!(records.iter().all(|record| record.name == "shared"));
    assert_ne!(records[0].id, records[1].id);

    let claimed = fixture
        .aliases
        .get(&LegacyAddressKey::parse("shared.md").expect("address"))
        .expect("alias")
        .expect("one of them claimed the address");
    assert!(records.iter().any(|record| record.id == claimed));
}

#[test]
fn two_sources_with_identical_content_stay_two_records() {
    // Identity is where the source was found, never what it contains. Collapsing byte-identical
    // files would silently delete one of the user's memories.
    let fixture = fixture("duplicate-content");
    let body = legacy_file("same", "Identical body.");
    fixture.write_legacy("copy-a.md", &body);
    fixture.write_legacy("copy-b.md", &body);

    let outcome = fixture.run();

    assert_eq!(outcome.migrated, 2);
    assert_eq!(fixture.records().len(), 2);
    assert_eq!(fixture.entry("copy-a.md").stage, MigrationStage::Completed);
    assert_eq!(fixture.entry("copy-b.md").stage, MigrationStage::Completed);
    assert_ne!(
        fixture.entry("copy-a.md").target_memory_id,
        fixture.entry("copy-b.md").target_memory_id
    );
}

#[test]
fn the_journal_keys_on_the_file_while_the_alias_keys_on_the_declared_name() {
    // The two identities answer different questions, and a file whose frontmatter disagrees with
    // its filename is the case that proves they are not the same value wearing two names.
    let fixture = fixture("name-mismatch");
    fixture.write_legacy("on-disk.md", &legacy_file("in-frontmatter", "Body."));

    fixture.run();

    let entry = fixture.entry("on-disk.md");
    assert_eq!(entry.stage, MigrationStage::Completed);
    assert!(entry.source_id.as_str().contains("on-disk.md"));
    assert_eq!(
        fixture
            .aliases
            .get(&LegacyAddressKey::parse("in-frontmatter.md").expect("address"))
            .expect("alias"),
        entry.target_memory_id
    );
    assert_eq!(
        fixture
            .aliases
            .get(&LegacyAddressKey::parse("on-disk.md").expect("address"))
            .expect("alias"),
        None
    );
}

#[test]
fn a_source_edited_after_discovery_is_never_overwritten_or_deleted() {
    let fixture = fixture("source-changed");
    fixture.seed("edited", "Original body.");
    // Stops with the original's fingerprint journalled and nothing else done.
    fixture.service.interrupt_after(MigrationStage::Discovered);
    fixture.run();

    fixture.write_legacy(
        "edited.md",
        &legacy_file("edited", "Edited outside VaneHub."),
    );
    let fixture = fixture.restart();
    let outcome = fixture.run();

    assert_eq!(outcome.source_changed, 1);
    assert_eq!(outcome.migrated, 0);
    assert!(outcome.requires_repair());
    assert!(fixture.legacy_exists("edited.md"));
    assert!(
        std::fs::read_to_string(fixture.root.join("edited.md"))
            .expect("source")
            .contains("Edited outside VaneHub."),
        "the user's edit must survive"
    );
    assert_eq!(
        fixture.entry("edited.md").stage,
        MigrationStage::SourceChanged
    );
}

#[test]
fn a_source_edited_after_its_backup_is_not_deleted_by_the_run_that_backed_it_up() {
    // The re-check immediately before the irreversible step. Without it, an edit made after the
    // backup would be destroyed by a delete the backup cannot restore.
    let fixture = fixture("changed-before-delete");
    fixture.seed("late-edit", "Original body.");
    fixture
        .service
        .interrupt_after(MigrationStage::ProjectionWritten);
    fixture.run();
    assert!(fixture.legacy_exists("late-edit.md"));

    fixture.write_legacy(
        "late-edit.md",
        &legacy_file("late-edit", "Edited after backup."),
    );
    let fixture = fixture.restart();
    let outcome = fixture.run();

    assert_eq!(outcome.source_changed, 1);
    assert!(fixture.legacy_exists("late-edit.md"));
    assert!(std::fs::read_to_string(fixture.root.join("late-edit.md"))
        .expect("source")
        .contains("Edited after backup."));
}

#[test]
fn a_backup_that_cannot_be_written_stops_before_anything_is_removed() {
    let fixture = fixture("backup-fails");
    fixture.seed("fragile", "Body.");
    fixture
        .sources
        .inject_failure(LegacyOperation::Backup, "fragile.md");

    let outcome = fixture.run();

    assert_eq!(outcome.failed, 1);
    assert!(fixture.legacy_exists("fragile.md"));
    assert!(fixture.records().is_empty());
    assert_eq!(fixture.entry("fragile.md").stage, MigrationStage::Failed);
}

#[test]
fn a_backup_that_does_not_reproduce_the_source_is_refused() {
    // A backup exists so a source an older build could read can be restored. One that does not
    // reproduce the original bytes cannot do that, and proceeding past it would make the delete
    // that follows unrecoverable.
    let fixture = fixture("backup-corrupt");
    fixture.seed("verified", "Body.");
    fixture
        .service
        .interrupt_after(MigrationStage::BackupWritten);
    fixture.run();

    std::fs::write(
        fixture.root.join("legacy-backup").join("verified.md"),
        "truncated",
    )
    .expect("corrupt the backup");
    let fixture = fixture.restart();
    let outcome = fixture.run();

    assert_eq!(outcome.failed, 1);
    assert!(outcome
        .failure_codes
        .contains(&"backup_verification_failed".to_string()));
    assert!(fixture.legacy_exists("verified.md"));
    assert!(fixture.records().is_empty());
}

#[test]
fn a_projection_failure_leaves_the_legacy_source_in_place() {
    // The projection is derived, but it is written before the source is removed on purpose: a
    // record that exists only as a file is invisible to every listing, and removing the original
    // then would leave the user with nothing they can see.
    let fixture = fixture("projection-fails");
    fixture.seed("indexed", "Body.");
    fixture.projection.set_failing(true);

    let outcome = fixture.run();

    assert_eq!(outcome.failed, 1);
    assert!(fixture.legacy_exists("indexed.md"));
    assert_eq!(fixture.entry("indexed.md").stage, MigrationStage::Failed);

    // And once the projection recovers, a fresh journal-clearing repair is what re-runs it — a
    // terminal entry is deliberately not retried on its own, so this asserts the state rather than
    // a silent self-heal.
    fixture.projection.set_failing(false);
    let second = fixture.run();
    assert_eq!(second.failed, 1);
    assert!(second
        .failure_codes
        .contains(&"projection_injected_failure".to_string()));
}

#[test]
fn a_content_body_beyond_the_governed_bound_fails_without_losing_the_source() {
    // v1 had no content bound; v2 does. The oversized file is refused rather than truncated, and it
    // stays on disk where a human can decide what to do with it.
    let fixture = fixture("oversized");
    let huge = "x".repeat(40_000);
    fixture.write_legacy("huge.md", &legacy_file("huge", &huge));

    let outcome = fixture.run();

    assert_eq!(outcome.failed, 1);
    assert!(outcome
        .failure_codes
        .contains(&"domain_validation_failed".to_string()));
    assert!(fixture.legacy_exists("huge.md"));
    assert!(fixture.records().is_empty());
}

#[test]
fn a_held_directory_defers_the_run_instead_of_marking_anything_failed() {
    let fixture = fixture("busy");
    fixture.seed("waiting", "Body.");
    let _held = fixture.lock.try_acquire().expect("hold the directory");

    let outcome = fixture.run();

    assert_eq!(outcome.deferred, 1);
    assert_eq!(outcome.failed, 0);
    assert!(!outcome.requires_repair());
    assert!(fixture.legacy_exists("waiting.md"));
    // Journalled as discovered, not failed: nothing failed, it simply has not happened yet.
    assert_eq!(
        fixture.entry("waiting.md").stage,
        MigrationStage::Discovered
    );
}

#[test]
fn each_journal_boundary_resumes_into_exactly_one_record() {
    // The point of journalling every transition before the action it authorizes. Each boundary is
    // exercised on its own, with one source, because the property is per-boundary recoverability.
    for stage in INTERRUPT_POINTS {
        let label = format!("interrupt-{}", stage.as_str());
        let fixture = fixture(&label);
        fixture.seed("resumed", "Body.");
        let original = std::fs::read(fixture.root.join("resumed.md")).expect("original");

        fixture.service.interrupt_after(*stage);
        let first = fixture.run();
        assert_eq!(first.migrated, 0, "{label}: the run stopped at {stage:?}");
        assert_eq!(
            fixture.entry("resumed.md").stage,
            *stage,
            "{label}: the journal records where it stopped"
        );

        let fixture = fixture.restart();
        let second = fixture.run();

        assert_eq!(second.migrated, 1, "{label}: the resumed run completes it");
        let records = fixture.records();
        assert_eq!(records.len(), 1, "{label}: exactly one record, never two");
        assert_eq!(records[0].content, "Body.", "{label}");
        assert_eq!(records[0].revision, 1, "{label}: never re-written");
        assert!(!fixture.legacy_exists("resumed.md"), "{label}");
        assert_eq!(
            fixture.backup_bytes("resumed.md"),
            original,
            "{label}: the backup still reproduces the original"
        );
        assert_eq!(
            fixture.entry("resumed.md").stage,
            MigrationStage::Completed,
            "{label}"
        );
    }
}

#[test]
fn repeated_runs_over_a_finished_migration_change_nothing() {
    // Startup runs this on every launch. A second pass must be a no-op rather than a re-import.
    let fixture = fixture("repeated");
    fixture.seed("stable", "Body.");
    assert_eq!(fixture.run().migrated, 1);
    let first = fixture.records();

    let fixture = fixture.restart();
    let second_run = fixture.run();
    let fixture = fixture.restart();
    let third_run = fixture.run();

    // Nothing is discovered any more, because the source is gone and the backup lives in a
    // directory enumeration skips.
    assert_eq!(second_run.discovered, 0);
    assert_eq!(third_run.discovered, 0);
    assert_eq!(fixture.records(), first);
    assert_eq!(fixture.journal.list_all().expect("journal").len(), 1);
}

#[test]
fn two_migrators_over_one_directory_converge_on_one_record() {
    // Both claim the same journal row, so the loser adopts the winner's target id instead of
    // allocating a second one — which is what stops one source becoming two records.
    let fixture = fixture("racing");
    fixture.seed("contested", "Body.");
    fixture.service.interrupt_after(MigrationStage::Discovered);
    fixture.run();
    let claimed = fixture.entry("contested.md").target_memory_id.clone();
    assert!(claimed.is_some());

    // A second migrator over the same directory and the same database.
    let second = reopen(fixture.directory_path.clone(), None);
    let outcome = second.run();

    assert_eq!(outcome.migrated, 1);
    assert_eq!(second.entry("contested.md").target_memory_id, claimed);
    // And the first migrator, resumed, finds nothing left to do rather than importing a second
    // copy: the source is gone and its journal entry is terminal.
    let first_again = reopen(fixture.directory_path.clone(), None);
    let resumed = first_again.run();
    assert_eq!(resumed.discovered, 0);
    assert_eq!(resumed.migrated, 0);
    assert_eq!(resumed.failed, 0);
    assert_eq!(first_again.records().len(), 1);
    let entries = first_again.journal.list_all().expect("journal");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].stage, MigrationStage::Completed);
}

#[test]
fn a_source_removed_before_it_was_migrated_is_reported_rather_than_called_done() {
    // Something outside this migration deleted the file. There is no backup to restore from at
    // this stage, so saying "migrated" about text nobody can produce would be a lie.
    let fixture = fixture("source-vanished");
    fixture.seed("vanishing", "Body.");
    fixture.service.interrupt_after(MigrationStage::Discovered);
    fixture.run();

    std::fs::remove_file(fixture.root.join("vanishing.md")).expect("delete out of band");
    let fixture = fixture.restart();
    let outcome = fixture.run();

    assert_eq!(outcome.discovered, 0);
    assert_eq!(outcome.migrated, 0);
    assert_eq!(outcome.failed, 1);
    assert!(outcome
        .failure_codes
        .contains(&"source_vanished".to_string()));
    assert_eq!(fixture.entry("vanishing.md").stage, MigrationStage::Failed);
}

#[test]
fn a_mixed_directory_migrates_what_it_can_and_reports_the_rest() {
    let fixture = fixture("mixed");
    fixture.seed("alpha", "First.");
    fixture.seed("bravo", "Second.");
    fixture.write_legacy("no-body.md", "---\nname: no-body\ndescription: d\n---\n\n");
    fixture.write_legacy("no-frontmatter.md", "Just a body.\n");
    fixture.write_legacy("MEMORY.md", "# Memory index\n");

    let outcome = fixture.run();

    assert_eq!(outcome.discovered, 4);
    assert_eq!(outcome.migrated, 2);
    assert_eq!(outcome.quarantined, 2);
    assert_eq!(outcome.failed, 0);
    let names: Vec<String> = fixture
        .records()
        .into_iter()
        .map(|record| record.name)
        .collect();
    assert_eq!(names, vec!["alpha".to_string(), "bravo".to_string()]);
    // The derived index is never a source and was left alone.
    assert!(fixture.legacy_exists("MEMORY.md"));
}

/// Deliberately one past the 200-file cap the previous scan applied.
///
/// That cap governed a destructive reset, so a migration that inherited it would silently leave the
/// 201st memory behind while reporting success.
#[test]
fn two_hundred_and_one_sources_all_migrate() {
    let fixture = fixture("boundary-201");
    for index in 0..201 {
        fixture.seed(&format!("memory-{index:04}"), &format!("Body {index}."));
    }

    let outcome = fixture.run();

    assert_eq!(outcome.discovered, 201);
    assert_eq!(outcome.migrated, 201);
    assert_eq!(fixture.records().len(), 201);
    assert_eq!(
        fixture.projection.projected_ids().expect("projected").len(),
        201
    );
    assert_eq!(fixture.journal.list_all().expect("journal").len(), 201);
    for index in 0..201 {
        assert!(!fixture.legacy_exists(&format!("memory-{index:04}.md")));
    }
}

#[test]
fn a_thousand_sources_all_migrate_and_resume_cleanly() {
    let fixture = fixture("scale-1000");
    for index in 0..1_000 {
        fixture.seed(&format!("memory-{index:04}"), &format!("Body {index}."));
    }

    let outcome = fixture.run();

    assert_eq!(outcome.discovered, 1_000);
    assert_eq!(outcome.migrated, 1_000);
    assert_eq!(fixture.records().len(), 1_000);

    // A second pass over a thousand finished sources is a no-op, not a thousand re-imports.
    let fixture = fixture.restart();
    let second = fixture.run();
    assert_eq!(second.discovered, 0);
    assert_eq!(fixture.records().len(), 1_000);
}

#[test]
fn a_failure_code_never_carries_a_path_or_a_body() {
    // These reach diagnostics and logs. A code that embedded a file name would put the user's
    // directory layout, and a code that embedded a message would put their text, into both.
    let fixture = fixture("codes");
    // Both kinds of thing a code must never carry: the user's directory layout, and a credential
    // that happened to be recorded in a workspace URI.
    fixture.write_legacy(
        "secret-project-notes.md",
        &legacy_file("huge", &"x".repeat(40_000)).replace(
            "D:/code/vanehub-ai",
            "ssh://alice:hunter2@host.example.test/srv/secret",
        ),
    );

    let outcome = fixture.run();

    assert!(!outcome.failure_codes.is_empty());
    for code in &outcome.failure_codes {
        assert!(
            code.chars()
                .all(|character| character.is_ascii_lowercase() || character == '_'),
            "{code:?} must be a code, not prose"
        );
        assert!(!code.contains("secret-project-notes"));
        assert!(!code.contains("hunter2"));
    }
}

#[test]
fn a_stored_alias_is_not_replaced_by_a_later_migration() {
    // The first migrated memory of a given name keeps the old address. Re-pointing it would make a
    // pre-governance caller's next save land on a different memory than its last one.
    let fixture = fixture("alias-stability");
    fixture.write_legacy("first.md", &legacy_file("shared", "First."));
    fixture.run();
    let first_target = fixture
        .aliases
        .get(&LegacyAddressKey::parse("shared.md").expect("address"))
        .expect("alias");

    fixture.write_legacy("second.md", &legacy_file("shared", "Second."));
    let fixture = fixture.restart();
    fixture.run();

    assert_eq!(
        fixture
            .aliases
            .get(&LegacyAddressKey::parse("shared.md").expect("address"))
            .expect("alias"),
        first_target
    );
}

/// Kept next to the migration because the invariant is the migration's: nothing it writes may be
/// enumerated as a source on the next run.
#[test]
fn nothing_migration_writes_is_re_enumerated() {
    let fixture = fixture("no-reentry");
    fixture.seed("kept", "Body.");
    fixture.write_legacy("broken.md", "---\nname: broken\n");
    fixture.run();

    let leftovers = fixture
        .sources
        .enumerate_sources()
        .map(|sources| sources.len())
        .expect("enumerate");

    assert_eq!(
        leftovers, 0,
        "backups, quarantined files, and v2 records must all be invisible to the next scan"
    );
}
