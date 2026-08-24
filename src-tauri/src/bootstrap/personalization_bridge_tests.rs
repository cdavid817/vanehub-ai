//! The composition-boundary adapter, and the evidence for why it exists.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeZone, Utc};
use tempfile::TempDir;

use super::personalization_bridge::LegacyMemoryPortBridge;
use crate::contexts::agent_runtime::application::{AgentMemoryPort, MemorySource, SaveMemoryInput};
use crate::contexts::agent_runtime::domain::MemoryType as RuntimeMemoryType;
use crate::contexts::agent_runtime::infrastructure::FileAgentMemoryStore;
use crate::contexts::personalization::api::build_for_tests;
use crate::contexts::personalization::application::{
    ClockPort, CreateMemoryInput, MemoryApplicationService, MigrationStatePort,
    PersonalizationApplicationError, RetrievalIndexPort,
};
use crate::contexts::personalization::domain::{
    MemoryAudience, MemoryId, MemoryProvenance, MemoryRecord, MemoryScope, MemorySensitivity,
    MemorySource as GovernedSource, MemoryStatus, MemoryType as GovernedType, MigrationPhase,
    MigrationState,
};
use crate::contexts::personalization::infrastructure::SqliteMigrationState;
use crate::platform::database::NativeDatabase;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 24, 9, 0, 0).unwrap()
}

struct FixedClock;

impl ClockPort for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        now()
    }
}

#[derive(Default)]
struct FakeRetrievalIndex {
    ids: Mutex<Vec<MemoryId>>,
}

impl RetrievalIndexPort for FakeRetrievalIndex {
    fn upsert(&self, record: &MemoryRecord) -> Result<(), PersonalizationApplicationError> {
        let mut ids = self.ids.lock().expect("ids");
        if !ids.contains(&record.id) {
            ids.push(record.id.clone());
        }
        Ok(())
    }

    fn revoke(&self, id: &MemoryId) -> Result<(), PersonalizationApplicationError> {
        self.ids.lock().expect("ids").retain(|stored| stored != id);
        Ok(())
    }

    fn revoke_all(&self, ids: &[MemoryId]) -> Result<usize, PersonalizationApplicationError> {
        for id in ids {
            self.revoke(id)?;
        }
        Ok(ids.len())
    }

    fn indexed_ids(&self) -> Result<Vec<MemoryId>, PersonalizationApplicationError> {
        Ok(self.ids.lock().expect("ids").clone())
    }
}

struct Fixture {
    _directory: TempDir,
    data_root: std::path::PathBuf,
    memory_root: std::path::PathBuf,
    service: Arc<MemoryApplicationService>,
    migration_state: Arc<SqliteMigrationState>,
    bridge: LegacyMemoryPortBridge,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDir::with_prefix(format!("bridge-{label}-")).expect("temporary directory");
    let data_root = directory.path().to_path_buf();
    let database = NativeDatabase::new(data_root.clone()).expect("database");
    let memory_root = data_root.join("memory");
    let (api, service) = build_for_tests(
        memory_root.clone(),
        database.clone(),
        Arc::new(FakeRetrievalIndex::default()),
        Arc::new(FixedClock),
    );
    Fixture {
        _directory: directory,
        data_root,
        memory_root,
        service,
        migration_state: Arc::new(SqliteMigrationState::new(database)),
        bridge: LegacyMemoryPortBridge::new(api),
    }
}

fn mark_ready(fixture: &Fixture) {
    fixture
        .migration_state
        .save(&MigrationState {
            generation: 1,
            phase: MigrationPhase::Ready,
            started_at: Some(now()),
            completed_at: Some(now()),
            legacy_rows_migrated_at: Some(now()),
            last_error_code: None,
            repair_required: false,
        })
        .expect("mark migration complete");
}

fn seed(fixture: &Fixture, name: &str) -> MemoryRecord {
    fixture
        .service
        .create(CreateMemoryInput {
            name: name.to_string(),
            description: "seeded".to_string(),
            memory_type: GovernedType::Project,
            content: format!("content for {name}"),
            scope: MemoryScope::Global,
            audience: MemoryAudience::AllAgents,
            status: MemoryStatus::Active,
            source: GovernedSource::ExplicitUser,
            provenance: MemoryProvenance::default(),
            sensitivity: MemorySensitivity::Normal,
        })
        .expect("seed")
        .record
}

fn save(fixture: &Fixture, name: &str, content: &str) -> Result<(), String> {
    fixture
        .bridge
        .save(SaveMemoryInput {
            agent_id: "onepiece",
            folder: None,
            name: Some(name),
            description: Some("Package manager"),
            memory_type: Some(RuntimeMemoryType::Project),
            content,
            source: MemorySource::Explicit,
        })
        .map_err(|error| error.to_string())
}

#[test]
fn the_previous_file_store_silently_misreads_a_v2_directory() {
    // The evidence for why the bridge exists. Leaving the old store wired after migration does not
    // fail loudly — it half works, which is worse.
    let fixture = fixture("v1-misreads-v2");
    mark_ready(&fixture);
    seed(&fixture, "Plain name");
    let rich = seed(&fixture, "Ratio 1:2 rules");

    let legacy = FileAgentMemoryStore::new(&fixture.data_root).expect("legacy store");
    let seen = legacy.list_all().expect("legacy listing");

    // The colon-bearing name is legal in v2 — names are no longer filenames — but the v1 reader
    // rejects it as an invalid filename and drops the whole memory.
    assert_eq!(
        seen.len(),
        1,
        "the v1 reader silently drops a v2 memory whose name it cannot use as a filename"
    );
    assert_eq!(seen[0].name, "Plain name");
    // And what it does read, it reads incompletely: v2 writes `memory_type`, v1 looks for `type`.
    assert_eq!(
        seen[0].memory_type, None,
        "the v1 reader loses the type of a v2 record it otherwise accepts"
    );
    assert!(
        seen.iter().all(|memory| memory.id != rich.file_name()),
        "the dropped record is invisible rather than reported"
    );
}

#[test]
fn the_bridge_projects_the_compatibility_view_onto_the_old_port() {
    let fixture = fixture("projection");
    mark_ready(&fixture);
    let record = seed(&fixture, "Global");

    let seen = fixture.bridge.list_all().expect("bridge listing");
    assert_eq!(seen.len(), 1);
    assert_eq!(
        seen[0].id,
        record.file_name(),
        "the handle is the v2 file name"
    );
    assert_eq!(seen[0].name, "Global");
    assert_eq!(seen[0].content, "content for Global");
    assert_eq!(seen[0].memory_type, Some(RuntimeMemoryType::Project));
}

#[test]
fn the_bridge_fails_closed_until_migration_is_ready() {
    let fixture = fixture("fail-closed");
    seed(&fixture, "Present but unusable");
    assert!(
        fixture.bridge.list_all().expect("listing").is_empty(),
        "an incomplete migration yields no memories rather than a partial set"
    );

    mark_ready(&fixture);
    assert_eq!(fixture.bridge.list_all().expect("listing").len(), 1);
}

#[test]
fn a_bridge_save_writes_v2_and_never_a_v1_file() {
    let fixture = fixture("save");
    mark_ready(&fixture);
    save(&fixture, "Use npm", "Use npm for this repository.").expect("save");

    let files: Vec<String> = std::fs::read_dir(&fixture.memory_root)
        .expect("read directory")
        .flatten()
        .filter_map(|entry| entry.file_name().to_str().map(String::from))
        .filter(|name| name.ends_with(".md") && name != "MEMORY.md")
        .collect();
    assert_eq!(files.len(), 1, "exactly one authoritative file was written");
    // A v1 write would have produced a name-derived filename beside the v2 one.
    assert!(
        !files.iter().any(|name| name.contains("Use")),
        "no name-derived v1 file may be created: {files:?}"
    );
}

#[test]
fn saving_an_existing_name_updates_in_place_as_it_did_before() {
    let fixture = fixture("save-existing");
    mark_ready(&fixture);
    save(&fixture, "Use npm", "First version.").expect("save");
    let first = fixture.bridge.list_all().expect("listing");
    save(&fixture, "Use npm", "Second version.").expect("save");
    let second = fixture.bridge.list_all().expect("listing");

    assert_eq!(
        second.len(),
        1,
        "a repeated name must not add a second memory"
    );
    assert_eq!(second[0].id, first[0].id, "the same record was updated");
    assert_eq!(second[0].content, "Second version.");
}

#[test]
fn an_empty_save_is_refused_without_writing_anything() {
    let fixture = fixture("empty-save");
    mark_ready(&fixture);
    assert!(save(&fixture, "Blank", "   ").is_err());
    assert!(fixture.bridge.list_all().expect("listing").is_empty());
}

#[test]
fn deleting_uses_the_handle_the_listing_handed_out() {
    let fixture = fixture("delete");
    mark_ready(&fixture);
    let record = seed(&fixture, "Subject");
    let listed = fixture.bridge.list_all().expect("listing");
    assert_eq!(listed[0].id, record.file_name());

    fixture.bridge.delete(&listed[0].id).expect("delete");
    assert!(fixture.bridge.list_all().expect("listing").is_empty());
    assert!(fixture
        .service
        .detail(&record.id)
        .expect("detail")
        .is_none());
}

#[test]
fn delete_all_clears_the_view() {
    let fixture = fixture("delete-all");
    mark_ready(&fixture);
    for index in 0..3 {
        seed(&fixture, &format!("Global {index}"));
    }
    fixture.bridge.delete_all().expect("delete all");
    assert!(fixture.bridge.list_all().expect("listing").is_empty());
}
