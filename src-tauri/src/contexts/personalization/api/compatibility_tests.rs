//! The pre-governance compatibility view and its stable legacy-name alias.
//!
//! Exercised through `PersonalizationApi` alone. The adapter that satisfies another context's port
//! lives in `bootstrap` and is tested there, so nothing here needs to know a consumer exists.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeZone, Utc};
use tempfile::TempDir;

use super::{build_for_tests, CompatibilitySaveInput, PersonalizationApi};
use crate::contexts::personalization::application::{
    ClockPort, CreateMemoryInput, MemoryApplicationService, MigrationJournalPort,
    MigrationStatePort, PersonalizationApplicationError, RetrievalIndexPort, UpdateMemoryPatch,
};
use crate::contexts::personalization::domain::{
    LegacySourceId, MemoryAudience, MemoryId, MemoryProvenance, MemoryRecord, MemoryScope,
    MemorySensitivity, MemorySource, MemoryStatus, MemoryType, MigrationJournalEntry,
    MigrationStage, MigrationState, WorkspaceKey,
};
use crate::contexts::personalization::infrastructure::{
    SqliteMigrationJournal, SqliteMigrationState,
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

struct Fixture {
    /// `None` for a reopened stack: the original still owns the directory, and a second owner
    /// would delete it out from under the test when the first one dropped.
    _directory: Option<TempDir>,
    directory_path: std::path::PathBuf,
    api: PersonalizationApi,
    service: Arc<MemoryApplicationService>,
    journal: Arc<SqliteMigrationJournal>,
    migration_state: Arc<SqliteMigrationState>,
}

fn fixture(label: &str) -> Fixture {
    let directory =
        TempDir::with_prefix(format!("personalization-compat-{label}-")).expect("temporary dir");
    let path = directory.path().to_path_buf();
    reopen(path, Some(directory))
}

/// Rebuilds the stack over an existing directory, standing in for an application restart.
fn reopen(directory_path: std::path::PathBuf, keep: Option<TempDir>) -> Fixture {
    let database = NativeDatabase::new(directory_path.clone()).expect("database");
    let (api, service) = build_for_tests(
        directory_path.join("memory"),
        database.clone(),
        Arc::new(FakeRetrievalIndex::default()),
        Arc::new(FixedClock),
    );
    Fixture {
        _directory: keep,
        directory_path,
        api,
        service,
        journal: Arc::new(SqliteMigrationJournal::new(database.clone())),
        migration_state: Arc::new(SqliteMigrationState::new(database)),
    }
}

fn mark_ready(fixture: &Fixture) {
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

fn seed(
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
            memory_type: MemoryType::Project,
            content: format!("content for {name}"),
            scope,
            audience,
            status: MemoryStatus::Active,
            source: MemorySource::ExplicitUser,
            provenance: MemoryProvenance::default(),
            sensitivity: MemorySensitivity::Normal,
        })
        .expect("seed")
        .record
}

fn save(
    fixture: &Fixture,
    name: &str,
    content: &str,
) -> Result<(), PersonalizationApplicationError> {
    fixture
        .api
        .save_compatibility_memory(CompatibilitySaveInput {
            agent_id: Some("onepiece".to_string()),
            workspace: None,
            name: name.to_string(),
            description: "Package manager".to_string(),
            memory_type: Some(MemoryType::Project),
            content: content.to_string(),
            is_automatic: false,
        })
        .map(|_| ())
}

fn legacy_id(name: &str) -> LegacySourceId {
    LegacySourceId::from_display_name(name).expect("legacy identity")
}

fn alias_target(fixture: &Fixture, name: &str) -> Option<MemoryId> {
    fixture
        .journal
        .get(&legacy_id(name))
        .expect("journal read")
        .and_then(|entry| entry.memory_id)
}

// --- compatibility view ------------------------------------------------------------------------

#[test]
fn the_view_exposes_only_active_global_all_agent_records() {
    let fixture = fixture("visibility");
    mark_ready(&fixture);
    seed(
        &fixture,
        "Global",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    seed(
        &fixture,
        "Workspace scoped",
        MemoryScope::Workspace {
            workspace_key: WorkspaceKey::parse("ws_1").expect("workspace"),
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

    let seen = fixture.api.compatibility_memories().expect("view");
    assert_eq!(
        seen.len(),
        1,
        "a caller that cannot express a scope must not receive a scoped record"
    );
    assert_eq!(seen[0].name, "Global");
}

#[test]
fn the_view_fails_closed_until_migration_is_ready() {
    let fixture = fixture("fail-closed");
    seed(
        &fixture,
        "Present but unusable",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    assert!(
        fixture
            .api
            .compatibility_memories()
            .expect("view")
            .is_empty(),
        "an incomplete migration yields no memories rather than a partial set"
    );

    mark_ready(&fixture);
    assert_eq!(fixture.api.compatibility_memories().expect("view").len(), 1);
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
        fixture
            .api
            .compatibility_memories()
            .expect("view")
            .is_empty(),
        "a generation whose derived state is known to be wrong must not be read from"
    );
}

#[test]
fn delete_all_clears_the_view_and_leaves_scoped_records_alone() {
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
            workspace_key: WorkspaceKey::parse("ws_1").expect("workspace"),
        },
        MemoryAudience::AllAgents,
    );

    assert_eq!(
        fixture
            .api
            .delete_all_compatibility_memories()
            .expect("delete all"),
        3
    );
    assert!(fixture
        .api
        .compatibility_memories()
        .expect("view")
        .is_empty());
    assert!(
        fixture
            .service
            .detail(&scoped.id)
            .expect("detail")
            .is_some(),
        "a record the caller could never see must not be deleted by its reset"
    );
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

    assert!(!fixture
        .api
        .delete_compatibility_memory("something-that-is-not-a-memory-id.md")
        .expect("tolerant delete"));
    assert_eq!(fixture.api.compatibility_memories().expect("view").len(), 1);
}

// --- legacy-name alias -------------------------------------------------------------------------

#[test]
fn a_first_save_persists_an_alias_to_the_record_it_created() {
    let fixture = fixture("alias-created");
    mark_ready(&fixture);
    save(&fixture, "Use npm", "First").expect("save");

    let view = fixture.api.compatibility_memories().expect("view");
    assert_eq!(alias_target(&fixture, "Use npm"), Some(view[0].id.clone()));
}

#[test]
fn an_alias_addresses_its_exact_record_even_when_another_shares_the_name() {
    // The case a name search cannot answer. Without the alias, picking either would silently
    // overwrite a memory the user did not mean.
    let fixture = fixture("alias-exact");
    mark_ready(&fixture);
    save(&fixture, "Use npm", "Aliased original").expect("save");
    let aliased = fixture.api.compatibility_memories().expect("view")[0]
        .id
        .clone();

    let duplicate = seed(
        &fixture,
        "Use npm",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    assert_eq!(fixture.api.compatibility_memories().expect("view").len(), 2);

    save(&fixture, "Use npm", "Updated through the alias").expect("save");

    let view = fixture.api.compatibility_memories().expect("view");
    assert_eq!(view.len(), 2, "no third record was created");
    assert_eq!(
        view.iter()
            .find(|memory| memory.id == aliased)
            .expect("the aliased record")
            .content,
        "Updated through the alias"
    );
    assert_eq!(
        fixture
            .service
            .detail(&duplicate.id)
            .expect("detail")
            .expect("still there")
            .content,
        "content for Use npm",
        "the record the alias does not point at is untouched"
    );
}

#[test]
fn two_same_named_records_without_an_alias_are_refused_rather_than_guessed_between() {
    let fixture = fixture("alias-ambiguous");
    mark_ready(&fixture);
    seed(
        &fixture,
        "Use npm",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    seed(
        &fixture,
        "Use npm",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );

    let error = save(&fixture, "Use npm", "Which one?").expect_err("ambiguity must be refused");
    assert_eq!(
        error,
        PersonalizationApplicationError::AmbiguousLegacyName { matches: 2 }
    );
    assert_eq!(
        fixture.api.compatibility_memories().expect("view").len(),
        2,
        "a refused save creates nothing"
    );
}

#[test]
fn a_single_pre_existing_match_is_adopted_and_gains_an_alias() {
    let fixture = fixture("alias-adopt");
    mark_ready(&fixture);
    let existing = seed(
        &fixture,
        "Use npm",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    assert_eq!(alias_target(&fixture, "Use npm"), None);

    save(&fixture, "Use npm", "Adopted").expect("save");

    assert_eq!(fixture.api.compatibility_memories().expect("view").len(), 1);
    assert_eq!(alias_target(&fixture, "Use npm"), Some(existing.id.clone()));
    assert_eq!(
        fixture
            .service
            .detail(&existing.id)
            .expect("detail")
            .expect("exists")
            .content,
        "Adopted"
    );
}

#[test]
fn an_alias_survives_a_restart() {
    let original = fixture("alias-restart");
    mark_ready(&original);
    save(&original, "Use npm", "Before restart").expect("save");
    let before = original.api.compatibility_memories().expect("view")[0]
        .id
        .clone();

    let reopened = reopen(original.directory_path.clone(), None);
    save(&reopened, "Use npm", "After restart").expect("save");

    let view = reopened.api.compatibility_memories().expect("view");
    assert_eq!(view.len(), 1, "the alias survived, so nothing duplicated");
    assert_eq!(view[0].id, before);
    assert_eq!(view[0].content, "After restart");
}

#[test]
fn an_alias_pointing_at_a_deleted_record_does_not_block_the_name() {
    let fixture = fixture("alias-dangling");
    mark_ready(&fixture);
    save(&fixture, "Use npm", "Original").expect("save");
    let original = fixture.api.compatibility_memories().expect("view")[0].clone();

    fixture
        .api
        .delete_compatibility_memory(&original.file_name)
        .expect("delete");
    assert!(fixture
        .api
        .compatibility_memories()
        .expect("view")
        .is_empty());

    save(&fixture, "Use npm", "Recreated").expect("save");
    let view = fixture.api.compatibility_memories().expect("view");
    assert_eq!(view.len(), 1);
    assert_ne!(view[0].id, original.id, "a new record, not the dead one");
    assert_eq!(
        alias_target(&fixture, "Use npm"),
        Some(view[0].id.clone()),
        "the stale alias was replaced rather than left dangling"
    );
}

#[test]
fn an_alias_recorded_before_its_record_was_verified_does_not_resolve() {
    // A journal row can exist while the v2 file is written but unproven. Addressing that record
    // would hand a caller something that might be torn.
    let fixture = fixture("alias-unverified");
    mark_ready(&fixture);
    let record = seed(
        &fixture,
        "Use npm",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    fixture
        .journal
        .upsert(
            &MigrationJournalEntry {
                legacy_source_id: legacy_id("Some other name"),
                memory_id: Some(record.id.clone()),
                stage: MigrationStage::V2Written,
                legacy_backup_path: None,
                legacy_content_hash: None,
                last_error_code: None,
            },
            now(),
        )
        .expect("journal write");

    save(&fixture, "Some other name", "Created").expect("save");

    assert_eq!(fixture.api.compatibility_memories().expect("view").len(), 2);
    assert_eq!(
        fixture
            .service
            .detail(&record.id)
            .expect("detail")
            .expect("exists")
            .content,
        "content for Use npm",
        "the unverified alias target was not written through"
    );
}

#[test]
fn a_repeated_save_does_not_accumulate_alias_rows() {
    let fixture = fixture("alias-idempotent");
    mark_ready(&fixture);
    for index in 0..4 {
        save(&fixture, "Use npm", &format!("Version {index}")).expect("save");
    }
    assert_eq!(fixture.journal.list_all().expect("journal").len(), 1);
    assert_eq!(fixture.api.compatibility_memories().expect("view").len(), 1);
}

#[test]
fn identical_content_under_two_names_stays_two_records_with_two_aliases() {
    // Deduplicating on content would merge two legacy sources that were always distinct.
    let fixture = fixture("alias-duplicate-content");
    mark_ready(&fixture);
    save(&fixture, "First name", "Exactly the same body").expect("save");
    save(&fixture, "Second name", "Exactly the same body").expect("save");

    assert_eq!(fixture.api.compatibility_memories().expect("view").len(), 2);
    assert_eq!(fixture.journal.list_all().expect("journal").len(), 2);
    assert_ne!(
        alias_target(&fixture, "First name"),
        alias_target(&fixture, "Second name")
    );
}

#[test]
fn a_name_that_could_never_have_been_a_v1_filename_gets_no_alias() {
    let fixture = fixture("alias-impossible-name");
    mark_ready(&fixture);
    save(&fixture, "Ratio 1:2 rules", "Body").expect("save");

    assert!(
        fixture.journal.list_all().expect("journal").is_empty(),
        "nothing under v1 could have created this memory, so it has no legacy identity"
    );
    assert_eq!(fixture.api.compatibility_memories().expect("view").len(), 1);
}

#[test]
fn an_alias_update_advances_exactly_one_revision_and_a_stale_write_is_refused() {
    let fixture = fixture("alias-revision");
    mark_ready(&fixture);
    save(&fixture, "Use npm", "First").expect("save");
    let id = fixture.api.compatibility_memories().expect("view")[0]
        .id
        .clone();
    assert_eq!(
        fixture
            .service
            .detail(&id)
            .expect("detail")
            .expect("exists")
            .revision,
        1
    );

    save(&fixture, "Use npm", "Second").expect("save");
    assert_eq!(
        fixture
            .service
            .detail(&id)
            .expect("detail")
            .expect("exists")
            .revision,
        2
    );

    let conflict = fixture
        .service
        .update(
            &id,
            1,
            UpdateMemoryPatch {
                content: Some("Stale".to_string()),
                ..UpdateMemoryPatch::default()
            },
        )
        .expect_err("a stale revision must be refused");
    assert!(matches!(
        conflict,
        PersonalizationApplicationError::RevisionConflict(_)
    ));
}
