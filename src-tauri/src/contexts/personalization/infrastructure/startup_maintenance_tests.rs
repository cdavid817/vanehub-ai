//! Startup maintenance: what has to be true before memory may be used at all.
//!
//! Assembled from the real store, the real journal, and the real SQLite state, because every
//! property here is about durable state surviving a process boundary. The two collaborators that
//! belong to other contexts — the pre-file row conversion and the pre-governance settings — are
//! fakes, since what is under test is the order they run in and how often, not their internals.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeZone, Utc};
use tempfile::TempDir;

use super::{
    DurableMemoryHealth, FileLegacyMemorySource, MaintenanceGate, MarkdownDerivedIndex,
    MarkdownMemoryRepository, SqliteLegacyAddressAlias, SqliteLegacyPolicyMigration,
    SqliteMemoryProjection, SqliteMigrationJournal, SqliteMigrationState, SqlitePolicyRepository,
    UuidMemoryIdGenerator,
};
use crate::contexts::personalization::api::PersonalizationApi;
use crate::contexts::personalization::application::{
    ClockPort, DerivedIndexPort, LegacyMemoryMigrationPorts, LegacyMemoryMigrationService,
    LegacyPersonalizationSettings, LegacyPersonalizationSettingsPort, LegacyRowMigrationPort,
    LegacySettingField, LegacySettingsCompatibility, MaintenanceGatePort, MemoryApplicationService,
    MemoryHealthPort, MemoryProjectionPort, MemoryRepository, MigrationStatePort,
    PersonalizationApplicationError, PolicyRepository, ResetCounts, RetrievalIndexPort,
    StartupMaintenancePorts, StartupMaintenanceService, WorkspaceIdentityResolver,
};
use crate::contexts::personalization::domain::{
    MemoryId, MemoryPage, MemoryQuery, MemoryRecord, MemoryRuntimeHealth, MemoryScopeFilter,
    MemoryStatus, MigrationPhase, PersonalizationPolicyScope, PolicyToggle,
};
use crate::platform::database::NativeDatabase;

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 24, 9, 0, 0).unwrap()
}

struct FixedClock;

impl ClockPort for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        now()
    }
}

/// The pre-file row conversion, counted so "exactly once, ever" is assertable.
#[derive(Default)]
struct FakeRows {
    calls: AtomicUsize,
    /// v1 files this conversion writes, as `(file_name, contents)`.
    produces: Mutex<Vec<(String, String)>>,
    root: Mutex<Option<std::path::PathBuf>>,
    fails: AtomicBool,
}

impl LegacyRowMigrationPort for FakeRows {
    fn convert_rows_to_legacy_files(&self) -> Result<usize> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fails.load(Ordering::SeqCst) {
            return Err(PersonalizationApplicationError::Storage(
                "row_migration_failed".to_string(),
            ));
        }
        let root = self.root.lock().expect("root").clone();
        let produces = self.produces.lock().expect("produces").clone();
        let Some(root) = root else {
            return Ok(0);
        };
        for (file_name, contents) in &produces {
            std::fs::write(root.join(file_name), contents).expect("row conversion output");
        }
        Ok(produces.len())
    }
}

#[derive(Default)]
struct FakeLegacySettings {
    settings: Mutex<LegacyPersonalizationSettings>,
    reads: AtomicUsize,
    fails: AtomicBool,
}

impl LegacyPersonalizationSettingsPort for FakeLegacySettings {
    fn load(&self) -> Result<LegacyPersonalizationSettings> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if self.fails.load(Ordering::SeqCst) {
            return Err(PersonalizationApplicationError::Storage(
                "legacy_settings_unreadable".to_string(),
            ));
        }
        Ok(self.settings.lock().expect("settings").clone())
    }
}

/// The real projection with a switch that makes its writes fail.
struct SwitchableProjection {
    inner: SqliteMemoryProjection,
    fails: AtomicBool,
}

impl MemoryProjectionPort for SwitchableProjection {
    fn upsert(&self, record: &MemoryRecord, content_hash: &str) -> Result<()> {
        if self.fails.load(Ordering::SeqCst) {
            return Err(PersonalizationApplicationError::Storage(
                "projection_unavailable".to_string(),
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

struct SwitchableIndex {
    inner: MarkdownDerivedIndex,
    fails: AtomicBool,
}

impl DerivedIndexPort for SwitchableIndex {
    fn rebuild(&self, active: &[MemoryRecord]) -> Result<usize> {
        if self.fails.load(Ordering::SeqCst) {
            return Err(PersonalizationApplicationError::Storage(
                "index_unavailable".to_string(),
            ));
        }
        self.inner.rebuild(active)
    }
}

#[derive(Default)]
struct SwitchableRetrieval {
    ids: Mutex<Vec<MemoryId>>,
    fails: AtomicBool,
}

impl RetrievalIndexPort for SwitchableRetrieval {
    fn upsert(&self, record: &MemoryRecord) -> Result<()> {
        if self.fails.load(Ordering::SeqCst) {
            return Err(PersonalizationApplicationError::Storage(
                "retrieval_unavailable".to_string(),
            ));
        }
        let mut ids = self.ids.lock().expect("ids");
        if !ids.contains(&record.id) {
            ids.push(record.id.clone());
        }
        Ok(())
    }
    fn revoke(&self, id: &MemoryId) -> Result<()> {
        if self.fails.load(Ordering::SeqCst) {
            return Err(PersonalizationApplicationError::Storage(
                "retrieval_unavailable".to_string(),
            ));
        }
        self.ids.lock().expect("ids").retain(|stored| stored != id);
        Ok(())
    }
    fn revoke_all(&self, ids: &[MemoryId]) -> Result<usize> {
        for id in ids {
            self.revoke(id)?;
        }
        Ok(ids.len())
    }
    fn indexed_ids(&self) -> Result<Vec<MemoryId>> {
        Ok(self.ids.lock().expect("ids").clone())
    }
}

struct Fixture {
    _directory: Option<TempDir>,
    directory_path: std::path::PathBuf,
    root: std::path::PathBuf,
    api: PersonalizationApi,
    maintenance: Arc<StartupMaintenanceService>,
    state: Arc<SqliteMigrationState>,
    policies: Arc<SqlitePolicyRepository>,
    projection: Arc<SwitchableProjection>,
    index: Arc<SwitchableIndex>,
    retrieval: Arc<SwitchableRetrieval>,
    rows: Arc<FakeRows>,
    legacy_settings: Arc<FakeLegacySettings>,
    repository: Arc<MarkdownMemoryRepository>,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDir::with_prefix(format!("personalization-startup-{label}-"))
        .expect("temporary directory");
    let path = directory.path().to_path_buf();
    reopen(path, Some(directory), Arc::new(FakeRows::default()))
}

/// Rebuilds the whole stack over an existing directory: an application restart.
fn reopen(
    directory_path: std::path::PathBuf,
    keep: Option<TempDir>,
    rows: Arc<FakeRows>,
) -> Fixture {
    let database = NativeDatabase::new(directory_path.clone()).expect("database");
    let root = directory_path.join("memory");
    *rows.root.lock().expect("root") = Some(root.clone());

    let repository = Arc::new(
        MarkdownMemoryRepository::new(root.clone(), Arc::new(UuidMemoryIdGenerator))
            .expect("repository"),
    );
    let projection = Arc::new(SwitchableProjection {
        inner: SqliteMemoryProjection::new(database.clone()),
        fails: AtomicBool::new(false),
    });
    let index = Arc::new(SwitchableIndex {
        inner: MarkdownDerivedIndex::new(root.clone()),
        fails: AtomicBool::new(false),
    });
    let retrieval = Arc::new(SwitchableRetrieval::default());
    let memories = Arc::new(MemoryApplicationService::new(
        repository.clone(),
        repository.clone(),
        projection.clone(),
        index.clone(),
        retrieval.clone(),
        Arc::new(FixedClock),
    ));

    let policies = Arc::new(SqlitePolicyRepository::new(database.clone()));
    let aliases = Arc::new(SqliteLegacyAddressAlias::new(database.clone()));
    let state = Arc::new(SqliteMigrationState::new(database.clone()));
    let sources = Arc::new(
        FileLegacyMemorySource::new(root.clone(), repository.lock()).expect("legacy source"),
    );
    let migration = Arc::new(LegacyMemoryMigrationService::new(
        LegacyMemoryMigrationPorts {
            sources,
            repository: repository.clone(),
            projection: projection.clone(),
            journal: Arc::new(SqliteMigrationJournal::new(database.clone())),
            aliases: aliases.clone(),
            identity: Arc::new(WorkspaceIdentityResolver::for_this_platform()),
            ids: Arc::new(UuidMemoryIdGenerator),
            clock: Arc::new(FixedClock),
        },
    ));
    let legacy_settings = Arc::new(FakeLegacySettings::default());
    // One gate object shared by the orchestration and the boundary, exactly as bootstrap assembles
    // it: the exclusion has to be between those two, not between two copies that never meet.
    let gate: Arc<dyn MaintenanceGatePort> =
        Arc::new(MaintenanceGate::new(&root).expect("maintenance gate"));
    let maintenance = Arc::new(StartupMaintenanceService::new(StartupMaintenancePorts {
        gate: gate.clone(),
        state: state.clone(),
        policies: policies.clone(),
        policy_migration: Arc::new(SqliteLegacyPolicyMigration::new(database.clone())),
        legacy_settings: legacy_settings.clone(),
        rows: rows.clone(),
        memories: migration,
        derived: memories.clone(),
        clock: Arc::new(FixedClock),
    }));
    let api = PersonalizationApi::new(
        memories,
        gate,
        maintenance.clone(),
        Arc::new(LegacySettingsCompatibility::new(
            policies.clone(),
            Arc::new(FixedClock),
        )),
        aliases,
        Arc::new(WorkspaceIdentityResolver::for_this_platform()),
    );

    Fixture {
        _directory: keep,
        directory_path,
        root,
        api,
        maintenance,
        state,
        policies,
        projection,
        index,
        retrieval,
        rows,
        legacy_settings,
        repository,
    }
}

fn legacy_file(name: &str, body: &str) -> String {
    format!("---\nname: {name}\ndescription: About {name}\ntype: project\nsource: explicit\n---\n\n{body}\n")
}

impl Fixture {
    fn write_legacy(&self, file_name: &str, contents: &str) {
        std::fs::write(self.root.join(file_name), contents).expect("legacy fixture");
    }

    fn restart(self) -> Fixture {
        let path = self.directory_path.clone();
        reopen(path, self._directory, self.rows.clone())
    }

    fn memory_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .api
            .compatibility_memories()
            .expect("view")
            .into_iter()
            .map(|memory| memory.name)
            .collect();
        names.sort();
        names
    }

    /// Every id-addressed memory file in the directory, sorted. The derived index is not one.
    fn memory_file_names(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.root)
            .expect("read dir")
            .flatten()
            .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
            .filter(|name| name.ends_with(".md") && name != "MEMORY.md")
            .collect();
        names.sort();
        names
    }

    fn index_contents(&self) -> String {
        std::fs::read_to_string(self.root.join("MEMORY.md")).unwrap_or_default()
    }
}

// =================================================================================================
// Policy and bootstrap
// =================================================================================================

#[test]
fn a_fresh_installation_reaches_ready_with_a_default_policy() {
    let fixture = fixture("fresh");

    let health = fixture.maintenance.run();

    assert_eq!(health, MemoryRuntimeHealth::Ready { generation: 1 });
    assert!(fixture.api.memory_is_ready());
    let policy = fixture
        .policies
        .load(&PersonalizationPolicyScope::Global)
        .expect("load")
        .expect("a validated global policy exists");
    assert_eq!(policy.memory_read_mode(), PolicyToggle::Enabled);
}

#[test]
fn the_legacy_settings_become_the_policy_on_the_first_real_startup() {
    let fixture = fixture("policy-migration");
    *fixture.legacy_settings.settings.lock().expect("settings") = LegacyPersonalizationSettings {
        about_user: Some("Prefers concise answers.".to_string()),
        style_rules: Some("No preamble.".to_string()),
        custom_instructions_enabled: Some(true),
        memory_enabled: Some(false),
        tool_assisted_extraction_enabled: Some(false),
    };

    fixture.maintenance.run();

    let view = fixture.api.legacy_settings().expect("read-through");
    assert_eq!(
        view.settings.about_user.as_deref(),
        Some("Prefers concise answers.")
    );
    assert_eq!(view.settings.memory_enabled, Some(false));
    // The row and the marker moved together, so a second startup does not read the legacy settings
    // again — which is what stops an old value overwriting an edit made since.
    assert_eq!(fixture.legacy_settings.reads.load(Ordering::SeqCst), 1);
}

#[test]
fn a_failed_policy_migration_leaves_no_rows_and_no_marker() {
    let fixture = fixture("policy-rollback");
    fixture.legacy_settings.fails.store(true, Ordering::SeqCst);

    let health = fixture.maintenance.run();

    assert_eq!(health, MemoryRuntimeHealth::Failed);
    assert!(!fixture.api.memory_is_ready());
    assert_eq!(
        fixture.state.load().expect("state").phase,
        MigrationPhase::Failed
    );
    // Nothing was committed, so a later startup with readable settings still migrates them.
    fixture.legacy_settings.fails.store(false, Ordering::SeqCst);
    assert!(fixture.maintenance.run().allows_memory_use());
    assert_eq!(fixture.legacy_settings.reads.load(Ordering::SeqCst), 2);
}

#[test]
fn a_repeated_startup_migrates_nothing_a_second_time() {
    let fixture = fixture("repeated-startup");
    fixture.write_legacy("kept.md", &legacy_file("kept", "Body."));
    assert!(fixture.maintenance.run().allows_memory_use());
    let first = fixture.memory_names();
    let generation = fixture.state.load().expect("state").generation;

    let fixture = fixture.restart();
    let health = fixture.maintenance.run();

    assert_eq!(
        health,
        MemoryRuntimeHealth::Ready { generation },
        "a completed installation commits no new generation"
    );
    assert_eq!(fixture.memory_names(), first);
    assert_eq!(
        fixture.rows.calls.load(Ordering::SeqCst),
        1,
        "the row conversion runs exactly once, ever — the counter survives the restart"
    );
    assert_eq!(
        fixture.legacy_settings.reads.load(Ordering::SeqCst),
        0,
        "the second startup does not consult the legacy settings at all"
    );
}

#[test]
fn an_edited_policy_is_not_overwritten_by_the_legacy_settings_on_a_later_startup() {
    // The concrete regression: the legacy rows still hold their old values, and a migration that
    // re-ran would put them back on top of what the user has since chosen.
    let fixture = fixture("no-reapply");
    *fixture.legacy_settings.settings.lock().expect("settings") = LegacyPersonalizationSettings {
        memory_enabled: Some(true),
        ..LegacyPersonalizationSettings::default()
    };
    fixture.maintenance.run();
    let before = fixture.api.legacy_settings().expect("view");
    fixture
        .api
        .save_legacy_setting(LegacySettingField::MemoryEnabled(false), before.revision)
        .expect("user turns memory off");

    let fixture = fixture.restart();
    fixture.maintenance.run();

    assert_eq!(
        fixture
            .api
            .legacy_settings()
            .expect("view")
            .settings
            .memory_enabled,
        Some(false),
        "the user's edit survives a restart"
    );
}

// =================================================================================================
// The compatibility window
// =================================================================================================

#[test]
fn the_old_page_reads_through_and_writes_through_to_the_policy() {
    let fixture = fixture("read-write-through");
    fixture.maintenance.run();

    let before = fixture.api.legacy_settings().expect("read-through");
    let after = fixture
        .api
        .save_legacy_setting(
            LegacySettingField::AboutUser("Works on VaneHub.".to_string()),
            before.revision,
        )
        .expect("write-through");

    assert_eq!(
        after.settings.about_user.as_deref(),
        Some("Works on VaneHub.")
    );
    assert!(after.revision > before.revision);
    // And the policy itself changed, not a copy of it.
    assert_eq!(
        fixture
            .policies
            .load(&PersonalizationPolicyScope::Global)
            .expect("load")
            .expect("policy")
            .about_user(),
        "Works on VaneHub."
    );
}

#[test]
fn a_save_from_a_stale_screen_is_refused_with_a_typed_conflict() {
    let fixture = fixture("conflict");
    fixture.maintenance.run();
    let stale = fixture.api.legacy_settings().expect("view").revision;
    fixture
        .api
        .save_legacy_setting(
            LegacySettingField::AboutUser("First edit.".to_string()),
            stale,
        )
        .expect("first edit");

    let rejected = fixture.api.save_legacy_setting(
        LegacySettingField::AboutUser("Second edit from an old screen.".to_string()),
        stale,
    );

    match rejected {
        Err(PersonalizationApplicationError::RevisionConflict(conflict)) => {
            assert_eq!(conflict.expected, stale);
            assert!(conflict.current > stale);
        }
        other => panic!("expected a typed conflict, got {other:?}"),
    }
    // And the first edit stands: a refused save changes nothing.
    assert_eq!(
        fixture
            .api
            .legacy_settings()
            .expect("view")
            .settings
            .about_user
            .as_deref(),
        Some("First edit.")
    );
}

#[test]
fn the_legacy_memory_switch_moves_reading_and_saving_together() {
    // One switch governed both, so splitting them here would let the old page put the policy into a
    // state it cannot express and therefore cannot show the user.
    let fixture = fixture("memory-switch");
    fixture.maintenance.run();
    let revision = fixture.api.legacy_settings().expect("view").revision;

    fixture
        .api
        .save_legacy_setting(LegacySettingField::MemoryEnabled(false), revision)
        .expect("turn memory off");

    let policy = fixture
        .policies
        .load(&PersonalizationPolicyScope::Global)
        .expect("load")
        .expect("policy");
    assert_eq!(policy.memory_read_mode(), PolicyToggle::Disabled);
    assert_eq!(policy.explicit_save_mode(), PolicyToggle::Disabled);
    assert_eq!(policy.automatic_extraction_mode(), PolicyToggle::Disabled);
    // Global memory access is deliberately untouched: it is not what this switch ever meant.
    assert_eq!(policy.global_memory_access_mode(), PolicyToggle::Enabled);
}

// =================================================================================================
// Memory startup
// =================================================================================================

#[test]
fn an_empty_directory_reaches_ready() {
    let fixture = fixture("empty");

    assert!(fixture.maintenance.run().allows_memory_use());
    assert!(fixture.memory_names().is_empty());
}

#[test]
fn a_directory_of_v1_files_is_migrated_before_anything_is_ready() {
    let fixture = fixture("v1-only");
    fixture.write_legacy("alpha.md", &legacy_file("alpha", "First."));
    fixture.write_legacy("bravo.md", &legacy_file("bravo", "Second."));

    assert!(fixture.maintenance.run().allows_memory_use());

    assert_eq!(
        fixture.memory_names(),
        vec!["alpha".to_string(), "bravo".to_string()]
    );
    assert!(!fixture.root.join("alpha.md").exists());
    assert!(fixture.index_contents().contains("alpha"));
}

#[test]
fn rows_are_converted_before_the_files_that_conversion_produces() {
    // The ordering the whole orchestration exists for: two independent startup paths would race
    // over one directory, and the file migration would run before the rows had become files.
    let fixture = fixture("rows-first");
    *fixture.rows.produces.lock().expect("produces") = vec![(
        "from-row.md".to_string(),
        legacy_file("from-row", "Row body."),
    )];

    assert!(fixture.maintenance.run().allows_memory_use());

    assert_eq!(fixture.memory_names(), vec!["from-row".to_string()]);
    assert!(
        !fixture.root.join("from-row.md").exists(),
        "the intermediate v1 file is consumed, not left behind"
    );
}

#[test]
fn a_row_and_an_unrelated_v1_file_both_become_records() {
    let fixture = fixture("rows-plus-files");
    *fixture.rows.produces.lock().expect("produces") = vec![(
        "from-row.md".to_string(),
        legacy_file("from-row", "Row body."),
    )];
    fixture.write_legacy("on-disk.md", &legacy_file("on-disk", "File body."));

    fixture.maintenance.run();

    assert_eq!(
        fixture.memory_names(),
        vec!["from-row".to_string(), "on-disk".to_string()]
    );
}

#[test]
fn nothing_writes_a_v1_file_once_the_installation_is_ready() {
    // The dual-write this change exists to end. After a save through the compatibility surface the
    // directory holds v2 records and the derived index, and no name-derived v1 file.
    use crate::contexts::personalization::api::CompatibilitySaveInput;

    let fixture = fixture("no-v1-after-ready");
    fixture.maintenance.run();

    fixture
        .api
        .save_compatibility_memory(CompatibilitySaveInput {
            agent_id: Some("onepiece".to_string()),
            workspace: None,
            name: "saved-after-ready".to_string(),
            description: "d".to_string(),
            memory_type: None,
            content: "Body.".to_string(),
            is_automatic: false,
        })
        .expect("save");

    assert!(!fixture.root.join("saved-after-ready.md").exists());
    let remaining: Vec<String> = std::fs::read_dir(&fixture.root)
        .expect("read dir")
        .flatten()
        .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
        .filter(|name| name.ends_with(".md") && name != "MEMORY.md")
        .collect();
    assert_eq!(remaining.len(), 1);
    assert!(
        remaining[0].ends_with(".md") && remaining[0].len() > "saved-after-ready.md".len() - 1,
        "the one Markdown file is id-addressed, not name-addressed"
    );
    // And re-running maintenance does not treat it as a legacy source.
    assert!(fixture.maintenance.run().allows_memory_use());
    assert_eq!(
        fixture.memory_names(),
        vec!["saved-after-ready".to_string()]
    );
}

#[test]
fn a_malformed_file_is_quarantined_and_never_reaches_the_runtime_view() {
    let fixture = fixture("malformed");
    fixture.write_legacy("good.md", &legacy_file("good", "Body."));
    fixture.write_legacy("broken.md", "---\nname: broken\n");

    let health = fixture.maintenance.run();

    assert!(health.allows_memory_use());
    assert_eq!(fixture.memory_names(), vec!["good".to_string()]);
    assert!(fixture.root.join("quarantine").join("broken.md").exists());
    assert!(!fixture.index_contents().contains("broken"));
}

// =================================================================================================
// Derived rebuild
// =================================================================================================

#[test]
fn a_projection_that_cannot_be_rebuilt_keeps_memory_unavailable() {
    let fixture = fixture("projection-failure");
    fixture.write_legacy("subject.md", &legacy_file("subject", "Body."));
    fixture.projection.fails.store(true, Ordering::SeqCst);

    let health = fixture.maintenance.run();

    assert!(!health.allows_memory_use());
    // The authoritative file is not rolled back — it is the one thing that is definitely correct.
    assert!(fixture.root.join("subject.md").exists() || !fixture.memory_names().is_empty());
    assert!(!fixture.api.memory_is_ready());
}

#[test]
fn an_index_that_cannot_be_rewritten_sets_repair_required() {
    let fixture = fixture("index-failure");
    fixture.write_legacy("subject.md", &legacy_file("subject", "Body."));
    fixture.index.fails.store(true, Ordering::SeqCst);

    let health = fixture.maintenance.run();

    assert_eq!(health, MemoryRuntimeHealth::RepairRequired);
    assert!(!fixture.api.memory_is_ready());
    assert!(fixture.state.load().expect("state").repair_required);
}

#[test]
fn a_retrieval_index_that_cannot_reconcile_sets_repair_required() {
    let fixture = fixture("retrieval-failure");
    fixture.write_legacy("subject.md", &legacy_file("subject", "Body."));
    fixture.retrieval.fails.store(true, Ordering::SeqCst);

    let health = fixture.maintenance.run();

    assert_eq!(health, MemoryRuntimeHealth::RepairRequired);
    assert!(!fixture.api.memory_is_ready());
}

#[test]
fn repair_is_retried_on_the_next_startup_and_converges() {
    let fixture = fixture("repair-retry");
    fixture.write_legacy("subject.md", &legacy_file("subject", "Body."));
    fixture.index.fails.store(true, Ordering::SeqCst);
    assert_eq!(
        fixture.maintenance.run(),
        MemoryRuntimeHealth::RepairRequired
    );

    let fixture = fixture.restart();
    let health = fixture.maintenance.run();

    assert!(health.allows_memory_use(), "a recovered view converges");
    assert_eq!(fixture.memory_names(), vec!["subject".to_string()]);
    assert!(fixture.index_contents().contains("subject"));
    assert!(!fixture.state.load().expect("state").repair_required);
}

#[test]
fn a_completed_journal_entry_alone_does_not_make_the_installation_ready() {
    // The distinction the whole state machine exists for. Every source can be `Completed` while the
    // views a runtime actually reads are still wrong, and only the second question decides `Ready`.
    let fixture = fixture("entry-versus-global");
    fixture.write_legacy("subject.md", &legacy_file("subject", "Body."));
    fixture.index.fails.store(true, Ordering::SeqCst);

    fixture.maintenance.run();

    let journal_is_done = fixture
        .repository
        .get(
            &fixture
                .projection
                .projected_ids()
                .expect("projected")
                .into_iter()
                .next()
                .expect("one record"),
        )
        .expect("read")
        .is_some();
    assert!(journal_is_done, "the record itself converted successfully");
    assert!(
        !fixture.api.memory_is_ready(),
        "and the installation is still not ready"
    );
}

// =================================================================================================
// Concurrency and restart
// =================================================================================================

/// A maintenance lease held on another thread, standing in for another process.
///
/// Another *thread* rather than the test's own: the gate is re-entrant per thread on purpose, so a
/// lease taken here would wave this thread's own writes straight through and prove nothing. A
/// separate thread has its own re-entrance state and its own file handle, which is what a separate
/// process has.
struct ForeignMaintainer {
    release: std::sync::mpsc::Sender<()>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl ForeignMaintainer {
    fn hold(root: &std::path::Path) -> Self {
        let (release, wait) = std::sync::mpsc::channel::<()>();
        let (held, confirmed) = std::sync::mpsc::channel::<()>();
        let root = root.to_path_buf();
        let thread = std::thread::spawn(move || {
            let gate = MaintenanceGate::new(&root).expect("gate");
            let lease = gate
                .try_enter_maintenance()
                .expect("acquire")
                .expect("the foreign maintainer takes the gate");
            held.send(()).expect("signal held");
            let _ = wait.recv();
            drop(lease);
        });
        confirmed
            .recv()
            .expect("the gate is held before the test proceeds");
        Self {
            release,
            thread: Some(thread),
        }
    }
}

impl Drop for ForeignMaintainer {
    fn drop(&mut self) {
        let _ = self.release.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[test]
fn a_second_process_reports_busy_and_then_observes_ready() {
    let fixture = fixture("busy");
    fixture.write_legacy("subject.md", &legacy_file("subject", "Body."));
    let second = reopen(
        fixture.directory_path.clone(),
        None,
        Arc::new(FakeRows::default()),
    );

    let foreign = ForeignMaintainer::hold(&fixture.root);
    assert_eq!(second.maintenance.run(), MemoryRuntimeHealth::Busy);
    assert!(!second.api.memory_is_ready());
    // And it did not read the pre-v2 directory or half-build anything.
    assert!(second.root.join("subject.md").exists());
    assert!(second.memory_names().is_empty());

    drop(foreign);
    assert!(fixture.maintenance.run().allows_memory_use());

    // The second process re-reads the durable row and stops reporting busy without being told.
    assert!(
        second.maintenance.health().allows_memory_use(),
        "busy must never be permanent"
    );
    assert_eq!(second.memory_names(), vec!["subject".to_string()]);
}

#[test]
fn the_gate_is_released_when_its_holder_goes_away() {
    let fixture = fixture("lock-release");
    {
        let _foreign = ForeignMaintainer::hold(&fixture.root);
        assert_eq!(fixture.maintenance.run(), MemoryRuntimeHealth::Busy);
    }

    assert!(
        fixture.maintenance.run().allows_memory_use(),
        "a dropped lease frees the next run"
    );
}

#[test]
fn maintenance_started_elsewhere_refuses_an_ordinary_save_and_changes_nothing() {
    // The barrier this gate exists for. Health stays `Ready` throughout — the foreign holder never
    // touches the durable row in this test — so a refusal here can only come from the gate. A
    // writer that had merely checked health would sail past and mutate underneath a reconciliation
    // about to rebuild every derived view from an earlier snapshot.
    use crate::contexts::personalization::api::CompatibilitySaveInput;

    let fixture = fixture("barrier-save");
    fixture.write_legacy("subject.md", &legacy_file("subject", "Body."));
    fixture.maintenance.run();
    assert!(fixture.api.memory_is_ready());

    let before_ids = fixture.projection.projected_ids().expect("projected");
    let before_index = fixture.index_contents();
    let before_retrieval = fixture.retrieval.indexed_ids().expect("retrieval");
    let before_files = fixture.memory_file_names();

    let foreign = ForeignMaintainer::hold(&fixture.root);

    // Still `Ready`: exactly the stale reading a health-only check would have trusted.
    assert!(
        fixture.api.memory_is_ready(),
        "health is the stale value; the gate is what must refuse"
    );
    let refused = fixture
        .api
        .save_compatibility_memory(CompatibilitySaveInput {
            agent_id: Some("onepiece".to_string()),
            workspace: None,
            name: "written-during-maintenance".to_string(),
            description: "d".to_string(),
            memory_type: None,
            content: "Body.".to_string(),
            is_automatic: false,
        });
    assert!(
        matches!(
            refused,
            Err(PersonalizationApplicationError::MaintenanceBusy)
        ),
        "expected a typed busy, got {refused:?}"
    );

    // Nothing moved, on any surface.
    assert_eq!(fixture.memory_file_names(), before_files);
    assert_eq!(
        fixture.projection.projected_ids().expect("projected"),
        before_ids
    );
    assert_eq!(fixture.index_contents(), before_index);
    assert_eq!(
        fixture.retrieval.indexed_ids().expect("retrieval"),
        before_retrieval
    );

    // And the same write succeeds once the directory is free again.
    drop(foreign);
    fixture
        .api
        .save_compatibility_memory(CompatibilitySaveInput {
            agent_id: Some("onepiece".to_string()),
            workspace: None,
            name: "written-during-maintenance".to_string(),
            description: "d".to_string(),
            memory_type: None,
            content: "Body.".to_string(),
            is_automatic: false,
        })
        .expect("the retry succeeds");
    assert_eq!(fixture.memory_file_names().len(), before_files.len() + 1);
}

#[test]
fn maintenance_started_elsewhere_refuses_an_ordinary_delete_and_changes_nothing() {
    // The delete case is the one with teeth: a delete that slipped through would be undone by the
    // reconciliation inside that maintenance, from a snapshot taken before it — putting a memory
    // the user removed back into the projection and the index.
    let fixture = fixture("barrier-delete");
    fixture.write_legacy("subject.md", &legacy_file("subject", "Body."));
    fixture.maintenance.run();
    let handle = fixture
        .api
        .compatibility_memories()
        .expect("view")
        .first()
        .expect("one memory")
        .file_name
        .clone();
    let before_ids = fixture.projection.projected_ids().expect("projected");
    let before_index = fixture.index_contents();

    let foreign = ForeignMaintainer::hold(&fixture.root);

    assert!(matches!(
        fixture.api.delete_compatibility_memory(&handle),
        Err(PersonalizationApplicationError::MaintenanceBusy)
    ));
    assert!(matches!(
        fixture.api.delete_all_compatibility_memories(),
        Err(PersonalizationApplicationError::MaintenanceBusy)
    ));

    assert!(fixture.root.join(&handle).exists(), "the file survives");
    assert_eq!(
        fixture.projection.projected_ids().expect("projected"),
        before_ids
    );
    assert_eq!(fixture.index_contents(), before_index);

    drop(foreign);
    assert!(fixture
        .api
        .delete_compatibility_memory(&handle)
        .expect("the retry succeeds"));
}

#[test]
fn a_read_during_maintenance_fails_closed_to_the_empty_view() {
    // A read that slipped through would see a half-migrated directory and hand a caller a partial
    // set as the whole truth — the same failure the unavailable case already refuses.
    let fixture = fixture("barrier-read");
    fixture.write_legacy("subject.md", &legacy_file("subject", "Body."));
    fixture.maintenance.run();
    assert_eq!(fixture.memory_names(), vec!["subject".to_string()]);

    let foreign = ForeignMaintainer::hold(&fixture.root);
    assert!(fixture.api.memory_is_ready());
    assert!(
        fixture.memory_names().is_empty(),
        "a read during maintenance yields nothing rather than a partial set"
    );

    drop(foreign);
    assert_eq!(fixture.memory_names(), vec!["subject".to_string()]);
}

#[test]
fn ordinary_mutations_share_the_gate_while_maintenance_is_excluded() {
    // Why the admission is shared rather than exclusive: several ordinary writes may proceed
    // together — the directory lock is what serializes them — but none may overlap maintenance.
    let fixture = fixture("barrier-shared");
    fixture.maintenance.run();

    let gate = MaintenanceGate::new(&fixture.root).expect("gate");
    let admission = gate.enter_mutation().expect("admitted");

    let root = fixture.root.clone();
    let observed = std::thread::spawn(move || {
        let gate = MaintenanceGate::new(&root).expect("gate");
        let concurrent = gate.enter_mutation().is_ok();
        let maintenance_blocked = matches!(gate.try_enter_maintenance(), Ok(None));
        (concurrent, maintenance_blocked)
    })
    .join()
    .expect("prober");

    assert_eq!(
        observed,
        (true, true),
        "another mutation is admitted alongside; maintenance is not"
    );
    drop(admission);
}

#[test]
fn a_restart_keeps_the_same_memory_ids_and_the_same_generation() {
    let fixture = fixture("restart");
    fixture.write_legacy("subject.md", &legacy_file("subject", "Body."));
    fixture.maintenance.run();
    let ids_before = fixture.projection.projected_ids().expect("projected");
    let state_before = fixture.state.load().expect("state");

    let fixture = fixture.restart();
    let health = fixture.maintenance.run();

    assert_eq!(
        health,
        MemoryRuntimeHealth::Ready {
            generation: state_before.generation
        }
    );
    assert_eq!(
        fixture.projection.projected_ids().expect("projected"),
        ids_before
    );
    assert_eq!(fixture.state.load().expect("state"), state_before);
    assert!(fixture.index_contents().contains("subject"));
}

#[test]
fn health_read_from_the_durable_row_alone_agrees_with_the_orchestration() {
    // Two readers of one question must never disagree, so this asserts they are the same answer
    // rather than two implementations that happen to match today.
    let fixture = fixture("health-agreement");
    let durable = DurableMemoryHealth::new(fixture.state.clone());

    assert_eq!(durable.health(), fixture.maintenance.health());
    fixture.maintenance.run();
    assert_eq!(durable.health(), fixture.maintenance.health());
    assert!(durable.health().allows_memory_use());
}

// =================================================================================================
// Fail-closed at the runtime boundary
// =================================================================================================

#[test]
fn the_compatibility_surface_returns_nothing_and_refuses_writes_until_ready() {
    use crate::contexts::personalization::api::CompatibilitySaveInput;

    let fixture = fixture("fail-closed");
    fixture.write_legacy("subject.md", &legacy_file("subject", "Body."));

    // A read fails closed with an empty view rather than a partial one a caller would treat as the
    // whole truth.
    assert!(fixture.memory_names().is_empty());
    // A write is refused with a typed error rather than silently doing nothing, which would let a
    // caller believe a memory was saved.
    let refused = fixture
        .api
        .save_compatibility_memory(CompatibilitySaveInput {
            agent_id: Some("onepiece".to_string()),
            workspace: None,
            name: "too-early".to_string(),
            description: "d".to_string(),
            memory_type: None,
            content: "Body.".to_string(),
            is_automatic: false,
        });
    assert!(matches!(
        refused,
        Err(PersonalizationApplicationError::MaintenanceRequired)
    ));
    assert!(matches!(
        fixture.api.delete_compatibility_memory("anything.md"),
        Err(PersonalizationApplicationError::MaintenanceRequired)
    ));
    assert!(matches!(
        fixture.api.delete_all_compatibility_memories(),
        Err(PersonalizationApplicationError::MaintenanceRequired)
    ));
    // And nothing was written on the way past.
    assert!(fixture.root.join("subject.md").exists());
}

#[test]
fn instructions_stay_readable_while_memory_is_not() {
    // Policy and memory are different questions. A migration that has not finished converting files
    // says nothing about whether the user's instructions can be shown.
    let fixture = fixture("instructions-while-unready");
    fixture.write_legacy("subject.md", &legacy_file("subject", "Body."));
    fixture.index.fails.store(true, Ordering::SeqCst);
    fixture.maintenance.run();

    assert!(!fixture.api.memory_is_ready());
    assert!(
        fixture.api.legacy_settings().is_ok(),
        "the settings page keeps working while memory is unavailable"
    );
}
