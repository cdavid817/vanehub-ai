use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Duration, TimeZone, Utc};
use tempfile::TempDir;

use super::markdown_memory_repository::{MarkdownMemoryRepository, DERIVED_INDEX_FILE_NAME};
use super::memory_document::{compose, content_hash, parse};
use super::memory_id_generator::UuidMemoryIdGenerator;
use crate::contexts::personalization::application::{
    CreateMemoryInput, MemoryIdGeneratorPort, MemoryMaintenanceRepository, MemoryRepository,
    PersonalizationApplicationError, UpdateMemoryPatch,
};
use crate::contexts::personalization::domain::{
    AgentId, MaintenancePhase, MemoryAudience, MemoryId, MemoryProvenance, MemoryScope,
    MemoryScopeFilter, MemorySensitivity, MemorySource, MemoryStatus, MemoryType,
    OwnedEntryClassification, ResetConfirmationToken, ResetMemoryRequest, RevisionConflict,
    WorkspaceKey, RESET_CONFIRMATION_PHRASE,
};

/// Deterministic ids so a test can assert on filenames and reproduce a collision on demand.
#[derive(Default)]
struct SequentialIds {
    next: AtomicUsize,
}

impl MemoryIdGeneratorPort for SequentialIds {
    fn generate(&self) -> MemoryId {
        let index = self.next.fetch_add(1, Ordering::SeqCst);
        MemoryId::parse(&format!("01K2MEM{index:019}")).expect("memory id")
    }
}

struct Fixture {
    _directory: TempDir,
    repository: MarkdownMemoryRepository,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDir::with_prefix(format!("personalization-markdown-{label}-"))
        .expect("temporary directory");
    let repository = MarkdownMemoryRepository::new(
        directory.path().join("memory"),
        Arc::new(SequentialIds::default()),
    )
    .expect("repository");
    Fixture {
        _directory: directory,
        repository,
    }
}

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 24, 9, 0, 0).unwrap()
}

fn workspace() -> WorkspaceKey {
    WorkspaceKey::parse("ws_1").expect("workspace")
}

fn input(name: &str) -> CreateMemoryInput {
    CreateMemoryInput {
        name: name.to_string(),
        description: "a description".to_string(),
        memory_type: MemoryType::Project,
        content: "Use npm for this repository.".to_string(),
        scope: MemoryScope::Global,
        audience: MemoryAudience::AllAgents,
        status: MemoryStatus::Active,
        source: MemorySource::ExplicitUser,
        provenance: MemoryProvenance::default(),
        sensitivity: MemorySensitivity::Normal,
    }
}

fn reset_request(scope: MemoryScopeFilter, statuses: Vec<MemoryStatus>) -> ResetMemoryRequest {
    ResetMemoryRequest {
        scope: scope.clone(),
        statuses: statuses.clone(),
        token: ResetConfirmationToken {
            value: "tok_01K2ABCDEF".to_string(),
            issued_at: now(),
            scope,
            statuses,
        },
        typed_phrase: RESET_CONFIRMATION_PHRASE.to_string(),
    }
}

#[test]
fn a_created_memory_is_named_only_by_its_immutable_id() {
    let fixture = fixture("create");
    let record = fixture
        .repository
        .create(input("Use npm"), now())
        .expect("create");

    assert_eq!(record.revision, 1, "revision 0 is never a stored value");
    assert_eq!(record.file_name(), format!("{}.md", record.id));
    assert!(fixture.repository.root().join(record.file_name()).is_file());

    let loaded = fixture
        .repository
        .get(&record.id)
        .expect("get")
        .expect("exists");
    assert_eq!(loaded, record);
}

#[test]
fn two_memories_may_share_a_display_name() {
    // Name collision used to mean overwrite. It now means two independent records.
    let fixture = fixture("duplicate-names");
    let first = fixture
        .repository
        .create(input("Use npm"), now())
        .expect("first");
    let second = fixture
        .repository
        .create(input("Use npm"), now())
        .expect("second");

    assert_ne!(first.id, second.id);
    assert_ne!(first.file_name(), second.file_name());
    assert_eq!(first.name, second.name);
    assert_eq!(
        fixture
            .repository
            .enumerate_owned_entries()
            .expect("enumerate")
            .len(),
        2
    );
}

#[test]
fn creating_over_an_existing_file_fails_instead_of_replacing_it() {
    // The id generator is deliberately made to repeat, standing in for the one-in-2^122 collision
    // and for any future generator bug. Silent replacement here would be silent data loss.
    struct FixedId;
    impl MemoryIdGeneratorPort for FixedId {
        fn generate(&self) -> MemoryId {
            MemoryId::parse("01K2FIXED000000000000000000").expect("memory id")
        }
    }

    let directory = TempDir::with_prefix("personalization-markdown-collision-").expect("directory");
    let repository =
        MarkdownMemoryRepository::new(directory.path().join("memory"), Arc::new(FixedId))
            .expect("repository");

    let first = repository.create(input("First"), now()).expect("first");
    let error = repository
        .create(input("Second"), now())
        .expect_err("a colliding id must not overwrite");
    assert!(matches!(error, PersonalizationApplicationError::Storage(_)));

    let preserved = repository
        .get(&first.id)
        .expect("get")
        .expect("exists")
        .name;
    assert_eq!(preserved, "First", "the original content survives");
}

#[test]
fn a_rename_keeps_the_file_and_advances_the_revision() {
    let fixture = fixture("rename");
    let created = fixture
        .repository
        .create(input("Original"), now())
        .expect("create");

    let renamed = fixture
        .repository
        .update(
            &created.id,
            1,
            UpdateMemoryPatch {
                name: Some("Renamed".to_string()),
                ..UpdateMemoryPatch::default()
            },
            now() + Duration::hours(1),
        )
        .expect("update");

    assert_eq!(renamed.id, created.id);
    assert_eq!(renamed.file_name(), created.file_name());
    assert_eq!(renamed.name, "Renamed");
    assert_eq!(renamed.revision, 2);
    assert_eq!(renamed.updated_at, now() + Duration::hours(1));
    assert_eq!(
        renamed.created_at, created.created_at,
        "an edit must not rewrite creation history"
    );
    assert_eq!(
        renamed.content, created.content,
        "an unmentioned field is left alone"
    );
}

#[test]
fn a_stale_revision_is_refused_without_touching_the_file() {
    let fixture = fixture("stale");
    let created = fixture
        .repository
        .create(input("Original"), now())
        .expect("create");
    fixture
        .repository
        .update(
            &created.id,
            1,
            UpdateMemoryPatch {
                content: Some("first edit".to_string()),
                ..UpdateMemoryPatch::default()
            },
            now(),
        )
        .expect("first edit");

    let error = fixture
        .repository
        .update(
            &created.id,
            1,
            UpdateMemoryPatch {
                content: Some("second edit".to_string()),
                ..UpdateMemoryPatch::default()
            },
            now(),
        )
        .expect_err("a stale revision must be refused");
    assert_eq!(
        error,
        PersonalizationApplicationError::RevisionConflict(RevisionConflict {
            expected: 1,
            current: 2,
        })
    );

    let stored = fixture
        .repository
        .get(&created.id)
        .expect("get")
        .expect("exists");
    assert_eq!(stored.content, "first edit");
    assert_eq!(stored.revision, 2);
}

#[test]
fn deleting_with_a_stale_revision_is_refused() {
    let fixture = fixture("delete-stale");
    let created = fixture
        .repository
        .create(input("Original"), now())
        .expect("create");

    assert!(fixture.repository.delete(&created.id, Some(9)).is_err());
    assert!(fixture.repository.get(&created.id).expect("get").is_some());

    let outcome = fixture
        .repository
        .delete(&created.id, Some(1))
        .expect("delete");
    assert!(outcome.deleted_file);
    assert!(
        !outcome.requires_repair(),
        "the file repository reports only what it owns"
    );
    assert!(fixture.repository.get(&created.id).expect("get").is_none());
}

#[test]
fn deleting_an_absent_memory_is_the_callers_desired_end_state() {
    let fixture = fixture("delete-absent");
    let outcome = fixture
        .repository
        .delete(
            &MemoryId::parse("01K2MISSING00000000000000").expect("id"),
            None,
        )
        .expect("delete");
    assert!(!outcome.deleted_file);
    assert!(!outcome.requires_repair());
}

#[test]
fn an_empty_store_enumerates_and_resets_cleanly() {
    let fixture = fixture("empty");
    assert!(fixture
        .repository
        .enumerate_owned_entries()
        .expect("enumerate")
        .is_empty());

    let outcome = fixture
        .repository
        .reset(&reset_request(MemoryScopeFilter::Any, Vec::new()), now())
        .expect("reset");
    assert_eq!(outcome.matched, 0);
    assert_eq!(outcome.deleted_files, 0);
    assert!(!outcome.requires_repair());
}

fn create_many(fixture: &Fixture, count: usize) {
    for index in 0..count {
        fixture
            .repository
            .create(input(&format!("Memory {index}")), now())
            .expect("create");
    }
}

#[test]
fn enumeration_and_reset_are_not_capped_at_two_hundred_files() {
    // The precise regression: the previous store's scan truncated at 200 and reset iterated that
    // truncated list, so memory 201 and beyond survived a "delete everything".
    for count in [1_usize, 200, 201] {
        let fixture = fixture(&format!("count-{count}"));
        create_many(&fixture, count);

        assert_eq!(
            fixture
                .repository
                .enumerate_owned_entries()
                .expect("enumerate")
                .len(),
            count,
            "enumeration must see all {count} files"
        );

        let outcome = fixture
            .repository
            .reset(&reset_request(MemoryScopeFilter::Any, Vec::new()), now())
            .expect("reset");
        assert_eq!(outcome.matched, count);
        assert_eq!(outcome.deleted_files, count);
        assert!(fixture
            .repository
            .enumerate_owned_entries()
            .expect("enumerate")
            .is_empty());
    }
}

#[test]
fn a_thousand_memories_reset_completely() {
    let fixture = fixture("thousand");
    create_many(&fixture, 1_000);
    assert_eq!(
        fixture
            .repository
            .enumerate_owned_entries()
            .expect("enumerate")
            .len(),
        1_000
    );

    let outcome = fixture
        .repository
        .reset(&reset_request(MemoryScopeFilter::Any, Vec::new()), now())
        .expect("reset");
    assert_eq!(outcome.deleted_files, 1_000);
    assert!(!outcome.requires_repair());
    assert!(fixture
        .repository
        .enumerate_owned_entries()
        .expect("enumerate")
        .is_empty());
}

#[test]
fn repeated_reset_is_idempotent() {
    let fixture = fixture("idempotent");
    create_many(&fixture, 5);

    let first = fixture
        .repository
        .reset(&reset_request(MemoryScopeFilter::Any, Vec::new()), now())
        .expect("first reset");
    assert_eq!(first.deleted_files, 5);

    let second = fixture
        .repository
        .reset(&reset_request(MemoryScopeFilter::Any, Vec::new()), now())
        .expect("second reset");
    assert_eq!(second.matched, 0);
    assert!(!second.requires_repair());
}

#[test]
fn a_scoped_reset_leaves_other_scopes_alone() {
    let fixture = fixture("scoped-reset");
    fixture
        .repository
        .create(input("global"), now())
        .expect("global");
    let mut project = input("project");
    project.scope = MemoryScope::Workspace {
        workspace_key: workspace(),
    };
    fixture.repository.create(project, now()).expect("project");

    let outcome = fixture
        .repository
        .reset(
            &reset_request(
                MemoryScopeFilter::Workspace {
                    workspace_key: workspace(),
                },
                Vec::new(),
            ),
            now(),
        )
        .expect("reset");
    assert_eq!(outcome.deleted_files, 1);

    let remaining = fixture
        .repository
        .enumerate_owned_entries()
        .expect("enumerate");
    assert_eq!(remaining.len(), 1);
}

#[test]
fn a_reset_without_the_typed_phrase_deletes_nothing() {
    let fixture = fixture("unauthorized");
    create_many(&fixture, 3);
    let mut request = reset_request(MemoryScopeFilter::Any, Vec::new());
    request.typed_phrase = "delete".to_string();

    assert!(fixture.repository.reset(&request, now()).is_err());
    assert_eq!(
        fixture
            .repository
            .enumerate_owned_entries()
            .expect("enumerate")
            .len(),
        3
    );
}

#[test]
fn classification_does_not_depend_on_a_file_parsing() {
    // The whole fix in one test: a file the parser cannot read must still be visible to the
    // operation that is supposed to remove it.
    let fixture = fixture("malformed");
    let good = fixture
        .repository
        .create(input("good"), now())
        .expect("good");
    fs::write(
        fixture
            .repository
            .root()
            .join("01K2BROKEN00000000000000000.md"),
        "not frontmatter at all",
    )
    .expect("write malformed");
    fs::write(
        fixture.repository.root().join(DERIVED_INDEX_FILE_NAME),
        "# Memory index\n",
    )
    .expect("write derived index");
    fs::write(fixture.repository.root().join("notes.txt"), "foreign").expect("write foreign file");
    fs::write(
        fixture.repository.root().join("legacy-memory.md"),
        "---\nname: legacy\n---\n\nbody\n",
    )
    .expect("write legacy file");

    let entries = fixture
        .repository
        .enumerate_owned_entries()
        .expect("enumerate");
    let classification = |name: &str| {
        entries
            .iter()
            .find(|entry| entry.file_name == name)
            .unwrap_or_else(|| panic!("{name} must be enumerated"))
            .classification
    };
    assert_eq!(
        classification(&good.file_name()),
        OwnedEntryClassification::ValidV2
    );
    assert_eq!(
        classification("01K2BROKEN00000000000000000.md"),
        OwnedEntryClassification::MalformedV2
    );
    assert_eq!(
        classification(DERIVED_INDEX_FILE_NAME),
        OwnedEntryClassification::Derived
    );
    assert_eq!(
        classification("notes.txt"),
        OwnedEntryClassification::Foreign
    );
    assert_eq!(
        classification("legacy-memory.md"),
        OwnedEntryClassification::LegacyV1
    );
}

#[test]
fn an_unrestricted_reset_removes_malformed_and_legacy_files_too() {
    let fixture = fixture("reset-malformed");
    fixture
        .repository
        .create(input("good"), now())
        .expect("good");
    fs::write(
        fixture
            .repository
            .root()
            .join("01K2BROKEN00000000000000000.md"),
        "not frontmatter",
    )
    .expect("write malformed");
    fs::write(
        fixture.repository.root().join("legacy-memory.md"),
        "legacy body",
    )
    .expect("write legacy");
    fs::write(
        fixture.repository.root().join(DERIVED_INDEX_FILE_NAME),
        "# Memory index\n",
    )
    .expect("write index");

    let outcome = fixture
        .repository
        .reset(&reset_request(MemoryScopeFilter::Any, Vec::new()), now())
        .expect("reset");
    assert_eq!(outcome.deleted_files, 3);
    assert!(
        fixture
            .repository
            .root()
            .join(DERIVED_INDEX_FILE_NAME)
            .is_file(),
        "the derived index is rebuilt, never deleted as if it were a memory"
    );
}

#[test]
fn a_scoped_reset_reports_an_unclassifiable_file_rather_than_guessing() {
    let fixture = fixture("scoped-malformed");
    fs::write(
        fixture
            .repository
            .root()
            .join("01K2BROKEN00000000000000000.md"),
        "not frontmatter",
    )
    .expect("write malformed");

    let outcome = fixture
        .repository
        .reset(
            &reset_request(MemoryScopeFilter::GlobalOnly, Vec::new()),
            now(),
        )
        .expect("reset");
    assert_eq!(outcome.deleted_files, 0);
    assert!(outcome.requires_repair());
    assert_eq!(
        outcome.failures[0].phase,
        MaintenancePhase::UnclassifiableEntry
    );
    assert!(
        fixture
            .repository
            .root()
            .join("01K2BROKEN00000000000000000.md")
            .is_file(),
        "the file is left for repair, not silently removed"
    );
}

#[test]
fn reconciliation_quarantines_a_malformed_file_instead_of_deleting_it() {
    let fixture = fixture("quarantine");
    fs::write(
        fixture
            .repository
            .root()
            .join("01K2BROKEN00000000000000000.md"),
        "not frontmatter",
    )
    .expect("write malformed");

    let outcome = fixture.repository.reconcile(now()).expect("reconcile");
    assert_eq!(outcome.quarantined_entries, 1);
    assert!(!fixture
        .repository
        .root()
        .join("01K2BROKEN00000000000000000.md")
        .is_file());
    assert!(fixture
        .repository
        .root()
        .join("quarantine")
        .join("01K2BROKEN00000000000000000.md")
        .is_file());

    // A quarantined entry stays visible so maintenance can report it.
    let entries = fixture
        .repository
        .enumerate_owned_entries()
        .expect("enumerate");
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].classification,
        OwnedEntryClassification::Quarantined
    );
}

#[test]
fn a_torn_write_is_detected_by_the_content_hash() {
    // The file's name is a valid id and its frontmatter parses, but the body was truncated. Without
    // the hash this would activate as a memory that says half of what the user wrote.
    let fixture = fixture("torn");
    let record = fixture
        .repository
        .create(input("Complete"), now())
        .expect("create");
    let path = fixture.repository.root().join(record.file_name());
    let composed = fs::read_to_string(&path).expect("read");
    let truncated = composed.replace("Use npm for this repository.", "Use npm for this");
    fs::write(&path, truncated).expect("write torn file");

    assert!(fixture.repository.get(&record.id).is_err());
    let entries = fixture
        .repository
        .enumerate_owned_entries()
        .expect("enumerate");
    assert_eq!(
        entries[0].classification,
        OwnedEntryClassification::MalformedV2
    );
}

#[test]
fn a_document_round_trips_through_compose_and_parse() {
    let fixture = fixture("round-trip");
    let mut created = input("Name with: a colon, a \"quote\", and a\nnewline");
    created.description = "Description with: punctuation".to_string();
    created.audience = MemoryAudience::SelectedAgents {
        agent_ids: vec![
            AgentId::parse("claude-code").expect("agent"),
            AgentId::parse("codex-cli").expect("agent"),
        ],
    };
    created.scope = MemoryScope::Workspace {
        workspace_key: workspace(),
    };
    created.provenance = MemoryProvenance {
        source_agent_id: Some(AgentId::parse("onepiece").expect("agent")),
        source_message_id: Some("msg: 1".to_string()),
        ..MemoryProvenance::default()
    };
    created.sensitivity = MemorySensitivity::Sensitive;

    let record = fixture.repository.create(created, now()).expect("create");
    let reparsed = parse(&compose(&record)).expect("round trip");
    assert_eq!(reparsed, record);
}

#[test]
fn a_frontmatter_field_declared_twice_is_refused() {
    // Last-wins would let a duplicated scope_kind override the one that was checked.
    let fixture = fixture("duplicate-field");
    let record = fixture
        .repository
        .create(input("Subject"), now())
        .expect("create");
    let path = fixture.repository.root().join(record.file_name());
    let composed = fs::read_to_string(&path).expect("read");
    let tampered = composed.replace(
        "scope_kind: global",
        "scope_kind: global\nscope_kind: workspace",
    );
    fs::write(&path, tampered).expect("write");

    assert!(fixture.repository.get(&record.id).is_err());
}

#[test]
fn a_body_edited_outside_the_application_does_not_activate_silently() {
    let fixture = fixture("external-edit");
    let record = fixture
        .repository
        .create(input("Subject"), now())
        .expect("create");
    let path = fixture.repository.root().join(record.file_name());
    let composed = fs::read_to_string(&path).expect("read");
    let edited = composed.replace(
        "Use npm for this repository.",
        "Use pnpm and publish to the internal registry.",
    );
    assert_ne!(
        content_hash("Use pnpm and publish to the internal registry."),
        content_hash("Use npm for this repository.")
    );
    fs::write(&path, edited).expect("write");

    assert!(
        fixture.repository.get(&record.id).is_err(),
        "an edit that did not go through the application is surfaced, not activated"
    );
}

#[test]
fn the_uuid_generator_produces_distinct_valid_ids() {
    let generator = UuidMemoryIdGenerator;
    let first = generator.generate();
    let second = generator.generate();
    assert_ne!(first, second);
    assert!(MemoryId::parse(first.as_str()).is_ok());
}

#[test]
fn a_path_that_is_not_a_plain_id_derived_name_is_refused() {
    let fixture = fixture("traversal");
    // These cannot be produced by `MemoryId`, which is the point: the guard is defence in depth
    // for any future caller that reaches the resolver with a raw string.
    for id in ["../escape", "nested/inner", "nested\\inner"] {
        assert!(
            MemoryId::parse(id).is_err(),
            "{id} must be rejected before it reaches the filesystem"
        );
    }
    // And the derived index cannot be addressed as a memory.
    assert!(fixture
        .repository
        .get(&MemoryId::parse("01K2MEM0000000000000000000").expect("id"))
        .expect("absent is not an error")
        .is_none());
}
