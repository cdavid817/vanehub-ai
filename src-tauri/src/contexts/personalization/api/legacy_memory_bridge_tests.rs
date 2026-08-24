use std::fs;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeZone, Utc};
use tempfile::TempDir;

use super::legacy_memory_bridge::LegacyMemoryPortBridge;
use super::PersonalizationApi;
use crate::contexts::agent_runtime::application::{AgentMemoryPort, MemorySource, SaveMemoryInput};
use crate::contexts::agent_runtime::domain::MemoryType as RuntimeMemoryType;
use crate::contexts::agent_runtime::infrastructure::FileAgentMemoryStore;
use crate::contexts::personalization::application::{
    ClockPort, CreateMemoryInput, MemoryApplicationService, MigrationStatePort,
    PersonalizationApplicationError, RetrievalIndexPort,
};
use crate::contexts::personalization::domain::{
    MemoryAudience, MemoryId, MemoryProvenance, MemoryRecord, MemoryScope, MemorySensitivity,
    MemorySource as GovernedSource, MemoryStatus, MemoryType as GovernedType, MigrationState,
};
use crate::contexts::personalization::infrastructure::{
    MarkdownDerivedIndex, MarkdownMemoryRepository, SqliteMemoryProjection, SqliteMigrationJournal,
    SqliteMigrationState, UuidMemoryIdGenerator,
};
use crate::platform::database::NativeDatabase;

pub(super) fn now() -> DateTime<Utc> {
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

pub(super) struct Fixture {
    /// `None` for a reopened stack: the original fixture still owns the directory, and a second
    /// owner would delete it out from under the test when the first one dropped.
    pub(super) _directory: Option<TempDir>,
    pub(super) directory_path: std::path::PathBuf,
    pub(super) memory_root: std::path::PathBuf,
    pub(super) service: Arc<MemoryApplicationService>,
    pub(super) migration_state: Arc<SqliteMigrationState>,
    pub(super) journal: Arc<SqliteMigrationJournal>,
    pub(super) api: PersonalizationApi,
    pub(super) bridge: LegacyMemoryPortBridge,
}

/// Builds the whole governed stack over one temporary directory.
///
/// Assembly lives here, at the owning context's api edge, rather than in the consumer's test file:
/// a test that reached across a context boundary for concrete persistence would violate the
/// inward-dependency rule, and moving the assembly is the repair rather than an exemption.
pub(super) fn fixture(label: &str) -> Fixture {
    let directory =
        TempDir::with_prefix(format!("legacy-bridge-{label}-")).expect("temporary directory");
    let directory_path = directory.path().to_path_buf();
    reopen(directory_path.clone(), Some(directory))
}

/// Rebuilds the stack over an existing directory, standing in for an application restart.
pub(super) fn reopen(directory_path: std::path::PathBuf, keep: Option<TempDir>) -> Fixture {
    let database = NativeDatabase::new(directory_path.clone()).expect("database");
    let memory_root = directory_path.join("memory");

    let repository = Arc::new(
        MarkdownMemoryRepository::new(memory_root.clone(), Arc::new(UuidMemoryIdGenerator))
            .expect("repository"),
    );
    let projection = Arc::new(SqliteMemoryProjection::new(database.clone()));
    let derived = Arc::new(MarkdownDerivedIndex::new(memory_root.clone()));
    let migration_state = Arc::new(SqliteMigrationState::new(database.clone()));
    let journal = Arc::new(SqliteMigrationJournal::new(database));

    let service = Arc::new(MemoryApplicationService::new(
        repository.clone(),
        repository,
        projection,
        derived,
        Arc::new(FakeRetrievalIndex::default()),
        Arc::new(FixedClock),
    ));
    let api = PersonalizationApi::new(service.clone(), migration_state.clone(), journal.clone());
    let bridge = LegacyMemoryPortBridge::new(api.clone());

    Fixture {
        _directory: keep,
        directory_path,
        memory_root,
        service,
        migration_state,
        journal,
        api,
        bridge,
    }
}

/// Marks migration complete so the compatibility view stops failing closed.
pub(super) fn mark_ready(fixture: &Fixture) {
    fixture
        .migration_state
        .save(&MigrationState {
            generation: 1,
            started_at: Some(now()),
            completed_at: Some(now()),
            last_error_code: None,
            repair_required: false,
        })
        .expect("mark migration complete");
}

pub(super) fn seed(
    fixture: &Fixture,
    name: &str,
    scope: MemoryScope,
    audience: MemoryAudience,
) -> MemoryRecord {
    fixture
        .service
        .create(CreateMemoryInput {
            name: name.to_string(),
            description: "seeded".to_string(),
            memory_type: GovernedType::Project,
            content: format!("content for {name}"),
            scope,
            audience,
            status: MemoryStatus::Active,
            source: GovernedSource::ExplicitUser,
            provenance: MemoryProvenance::default(),
            sensitivity: MemorySensitivity::Normal,
        })
        .expect("seed")
        .record
}

#[test]
fn the_previous_file_store_silently_misreads_a_v2_directory() {
    // This is the evidence for why the bridge exists. Leaving the old store wired after migration
    // does not fail loudly — it half works, which is worse.
    let fixture = fixture("v1-misreads-v2");
    mark_ready(&fixture);
    seed(
        &fixture,
        "Plain name",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    let rich = seed(
        &fixture,
        "Ratio 1:2 rules",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );

    let legacy = FileAgentMemoryStore::new(fixture.memory_root.parent().expect("data root"))
        .expect("legacy store");
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
        legacy
            .list_all()
            .expect("legacy listing")
            .iter()
            .all(|memory| memory.id != rich.file_name()),
        "the dropped record is invisible rather than reported"
    );
}

#[test]
fn the_bridge_exposes_only_the_pre_governance_view() {
    let fixture = fixture("visibility");
    mark_ready(&fixture);
    let visible = seed(
        &fixture,
        "Global",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    seed(
        &fixture,
        "Workspace scoped",
        MemoryScope::Workspace {
            workspace_key: crate::contexts::personalization::domain::WorkspaceKey::parse("ws_1")
                .expect("workspace"),
        },
        MemoryAudience::AllAgents,
    );
    seed(
        &fixture,
        "Audience restricted",
        MemoryScope::Global,
        MemoryAudience::SelectedAgents {
            agent_ids: vec![
                crate::contexts::personalization::domain::AgentId::parse("codex-cli")
                    .expect("agent"),
            ],
        },
    );

    let seen = fixture.bridge.list_all().expect("bridge listing");
    assert_eq!(
        seen.len(),
        1,
        "a caller that cannot express a scope must not receive a scoped record"
    );
    assert_eq!(seen[0].name, "Global");
    assert_eq!(seen[0].id, visible.file_name());
    assert_eq!(seen[0].content, "content for Global");
}

#[test]
fn the_bridge_fails_closed_until_migration_is_ready() {
    let fixture = fixture("fail-closed");
    seed(
        &fixture,
        "Present but unusable",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );

    assert!(
        fixture.bridge.list_all().expect("listing").is_empty(),
        "an incomplete migration yields no memories rather than a partial set"
    );

    mark_ready(&fixture);
    assert_eq!(fixture.bridge.list_all().expect("listing").len(), 1);
}

#[test]
fn a_repair_required_generation_is_not_usable() {
    let fixture = fixture("repair-required");
    seed(
        &fixture,
        "Subject",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    fixture
        .migration_state
        .save(&MigrationState {
            generation: 1,
            started_at: Some(now()),
            completed_at: Some(now()),
            last_error_code: Some("derived_rebuild_failed".to_string()),
            repair_required: true,
        })
        .expect("save state");

    assert!(
        fixture.bridge.list_all().expect("listing").is_empty(),
        "a generation whose derived state is known to be wrong must not be read from"
    );
}

#[test]
fn a_bridge_save_writes_v2_and_never_a_v1_file() {
    let fixture = fixture("save");
    mark_ready(&fixture);

    fixture
        .bridge
        .save(SaveMemoryInput {
            agent_id: "onepiece",
            folder: None,
            name: Some("Use npm"),
            description: Some("Package manager"),
            memory_type: Some(RuntimeMemoryType::Project),
            content: "Use npm for this repository.",
            source: MemorySource::Explicit,
        })
        .expect("save");

    let files: Vec<String> = fs::read_dir(&fixture.memory_root)
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

    let seen = fixture.bridge.list_all().expect("listing");
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].name, "Use npm");
    assert_eq!(seen[0].memory_type, Some(RuntimeMemoryType::Project));
    assert_eq!(seen[0].agent_id, "onepiece");
}

#[test]
fn saving_an_existing_name_updates_in_place_as_it_did_before() {
    // The previous contract: saving under an existing name replaces that memory rather than adding
    // a second one. v2 no longer treats a name as identity, so the bridge is what preserves it.
    let fixture = fixture("save-existing");
    mark_ready(&fixture);
    let save = |content: &str| {
        fixture
            .bridge
            .save(SaveMemoryInput {
                agent_id: "onepiece",
                folder: None,
                name: Some("Use npm"),
                description: Some("Package manager"),
                memory_type: Some(RuntimeMemoryType::Project),
                content,
                source: MemorySource::Explicit,
            })
            .expect("save")
    };

    save("First version.");
    let first = fixture.bridge.list_all().expect("listing");
    save("Second version.");
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
    assert!(fixture
        .bridge
        .save(SaveMemoryInput {
            agent_id: "onepiece",
            folder: None,
            name: Some("Blank"),
            description: Some("Blank"),
            memory_type: None,
            content: "   ",
            source: MemorySource::Explicit,
        })
        .is_err());
    assert!(fixture.bridge.list_all().expect("listing").is_empty());
}

#[test]
fn deleting_uses_the_handle_the_listing_handed_out() {
    let fixture = fixture("delete");
    mark_ready(&fixture);
    let record = seed(
        &fixture,
        "Subject",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );

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
fn deleting_an_unknown_handle_is_the_callers_desired_end_state() {
    let fixture = fixture("delete-unknown");
    mark_ready(&fixture);
    seed(
        &fixture,
        "Kept",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );

    fixture
        .bridge
        .delete("something-that-is-not-a-memory-id.md")
        .expect("delete is tolerant");
    assert_eq!(fixture.bridge.list_all().expect("listing").len(), 1);
}

#[test]
fn delete_all_clears_the_compatibility_view_and_leaves_scoped_records_alone() {
    let fixture = fixture("delete-all");
    mark_ready(&fixture);
    for index in 0..3 {
        seed(
            &fixture,
            &format!("Global {index}"),
            MemoryScope::Global,
            MemoryAudience::AllAgents,
        );
    }
    let scoped = seed(
        &fixture,
        "Workspace scoped",
        MemoryScope::Workspace {
            workspace_key: crate::contexts::personalization::domain::WorkspaceKey::parse("ws_1")
                .expect("workspace"),
        },
        MemoryAudience::AllAgents,
    );

    fixture.bridge.delete_all().expect("delete all");
    assert!(fixture.bridge.list_all().expect("listing").is_empty());
    assert!(
        fixture
            .service
            .detail(&scoped.id)
            .expect("detail")
            .is_some(),
        "a record the caller could never see must not be deleted by its reset"
    );
}
