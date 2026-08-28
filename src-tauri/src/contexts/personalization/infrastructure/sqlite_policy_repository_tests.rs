use chrono::{TimeZone, Utc};
use tempfile::TempDir;

use super::sqlite_policy_repository::SqlitePolicyRepository;
use crate::contexts::personalization::application::{
    PersonalizationApplicationError, PolicyRepository,
};
use crate::contexts::personalization::domain::{
    AgentId, InstructionMergeMode, PatchPolicyResult, PersonalizationPolicyPatch,
    PersonalizationPolicyScope, PolicyLayerState, PolicyToggle, WorkspaceKey,
    INSTRUCTION_FIELD_MAX_CHARS,
};
use crate::platform::database::NativeDatabase;

struct Fixture {
    _directory: TempDir,
    repository: SqlitePolicyRepository,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDir::with_prefix(format!("personalization-policy-{label}-"))
        .expect("temporary directory");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    Fixture {
        _directory: directory,
        repository: SqlitePolicyRepository::new(database),
    }
}

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 24, 9, 0, 0).unwrap()
}

fn agent() -> AgentId {
    AgentId::parse("claude-code").expect("agent")
}

fn workspace() -> WorkspaceKey {
    WorkspaceKey::parse("ws_1").expect("workspace")
}

fn agent_scope() -> PersonalizationPolicyScope {
    PersonalizationPolicyScope::Agent { agent_id: agent() }
}

#[test]
fn migration_88_creates_the_personalization_tables_and_indexes() {
    let fixture = fixture("schema");
    let connection = fixture
        .repository
        .clone()
        .raw_connection_for_tests()
        .expect("connection");

    for table in [
        "personalization_policy_overrides",
        "personalization_memory_projection",
        "personalization_memory_candidates",
        "personalization_migration_state",
    ] {
        let present: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .expect("table lookup");
        assert_eq!(present, 1, "{table} must exist after migration 88");
    }

    for index in [
        "idx_personalization_policy_scope",
        "idx_personalization_memory_status_updated",
        "idx_personalization_memory_scope",
        "idx_personalization_memory_source_agent",
        "idx_personalization_memory_type",
        "idx_personalization_memory_keyset",
        "idx_personalization_candidate_status_created",
        "idx_personalization_candidate_target",
    ] {
        let present: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = ?1",
                [index],
                |row| row.get(0),
            )
            .expect("index lookup");
        assert_eq!(present, 1, "{index} must exist after migration 88");
    }

    // The singleton migration-state row is seeded so later code never has to branch on "no row".
    let generation: i64 = connection
        .query_row(
            "SELECT generation FROM personalization_migration_state WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .expect("migration state row");
    assert_eq!(generation, 0);
}

#[test]
fn seeding_the_default_global_policy_is_idempotent() {
    let fixture = fixture("seed");
    let first = fixture
        .repository
        .seed_default_global(now())
        .expect("seed once");
    assert_eq!(first.revision(), 0);
    assert_eq!(first.memory_read_mode(), PolicyToggle::Enabled);

    // A user edit between startups must survive the next startup's seeding.
    let patched = fixture
        .repository
        .patch(
            &PersonalizationPolicyScope::Global,
            Some(0),
            PersonalizationPolicyPatch {
                about_user: Some("I use Rust".to_string()),
                ..PersonalizationPolicyPatch::default()
            },
            now(),
        )
        .expect("patch");
    assert!(matches!(patched, PatchPolicyResult::Updated(_)));

    let second = fixture
        .repository
        .seed_default_global(now())
        .expect("seed again");
    assert_eq!(second.about_user(), "I use Rust");
    assert_eq!(second.revision(), 1, "seeding must not reset the revision");
    assert_eq!(fixture.repository.list_all().expect("list").len(), 1);
}

#[test]
fn a_patch_round_trips_through_storage() {
    let fixture = fixture("round-trip");
    fixture.repository.seed_default_global(now()).expect("seed");

    fixture
        .repository
        .patch(
            &agent_scope(),
            None,
            PersonalizationPolicyPatch {
                instruction_merge_mode: Some(InstructionMergeMode::Replace),
                about_user: Some("agent about".to_string()),
                style_rules: Some("agent style".to_string()),
                memory_read_mode: Some(PolicyToggle::Disabled),
                ..PersonalizationPolicyPatch::default()
            },
            now(),
        )
        .expect("create the Agent override");

    let loaded = fixture
        .repository
        .load(&agent_scope())
        .expect("load")
        .expect("the Agent row exists");
    assert_eq!(
        loaded.instruction_merge_mode(),
        InstructionMergeMode::Replace
    );
    assert_eq!(loaded.about_user(), "agent about");
    assert_eq!(loaded.style_rules(), "agent style");
    assert_eq!(loaded.memory_read_mode(), PolicyToggle::Disabled);
    // Unmentioned dimensions stay inherit, which is the whole point of a scoped patch.
    assert_eq!(loaded.explicit_save_mode(), PolicyToggle::Inherit);
    assert_eq!(loaded.revision(), 1);
    assert_eq!(loaded.scope(), &agent_scope());
}

#[test]
fn a_stale_expected_revision_returns_the_current_record_instead_of_writing() {
    let fixture = fixture("conflict");
    fixture.repository.seed_default_global(now()).expect("seed");
    fixture
        .repository
        .patch(
            &PersonalizationPolicyScope::Global,
            Some(0),
            PersonalizationPolicyPatch {
                about_user: Some("first".to_string()),
                ..PersonalizationPolicyPatch::default()
            },
            now(),
        )
        .expect("first save");

    let result = fixture
        .repository
        .patch(
            &PersonalizationPolicyScope::Global,
            Some(0),
            PersonalizationPolicyPatch {
                about_user: Some("second".to_string()),
                ..PersonalizationPolicyPatch::default()
            },
            now(),
        )
        .expect("second save resolves");

    match result {
        PatchPolicyResult::Conflict { current } => {
            assert_eq!(current.revision(), 1);
            assert_eq!(
                current.about_user(),
                "first",
                "the conflict must carry the stored record so the UI can compare"
            );
        }
        PatchPolicyResult::Updated(_) => panic!("a stale revision must not write"),
    }

    let stored = fixture
        .repository
        .load(&PersonalizationPolicyScope::Global)
        .expect("load")
        .expect("row");
    assert_eq!(stored.about_user(), "first");
    assert_eq!(stored.revision(), 1);
}

#[test]
fn independent_scopes_do_not_replace_each_other() {
    // The concrete regression this guards: whole-`AppSettings` saves, where writing one scope
    // republished every other field.
    let fixture = fixture("independent");
    fixture.repository.seed_default_global(now()).expect("seed");

    let workspace_scope = PersonalizationPolicyScope::Workspace {
        workspace_key: workspace(),
    };
    fixture
        .repository
        .patch(
            &agent_scope(),
            None,
            PersonalizationPolicyPatch {
                about_user: Some("agent text".to_string()),
                ..PersonalizationPolicyPatch::default()
            },
            now(),
        )
        .expect("agent save");
    fixture
        .repository
        .patch(
            &workspace_scope,
            None,
            PersonalizationPolicyPatch {
                about_user: Some("workspace text".to_string()),
                ..PersonalizationPolicyPatch::default()
            },
            now(),
        )
        .expect("workspace save");

    assert_eq!(
        fixture
            .repository
            .load(&agent_scope())
            .expect("load")
            .expect("row")
            .about_user(),
        "agent text"
    );
    assert_eq!(
        fixture
            .repository
            .load(&workspace_scope)
            .expect("load")
            .expect("row")
            .about_user(),
        "workspace text"
    );
    assert_eq!(
        fixture
            .repository
            .load(&PersonalizationPolicyScope::Global)
            .expect("load")
            .expect("row")
            .about_user(),
        "",
        "neither save may touch the global row"
    );
}

#[test]
fn an_invalid_patch_is_rejected_before_anything_is_written() {
    let fixture = fixture("invalid");
    fixture.repository.seed_default_global(now()).expect("seed");

    let error = fixture
        .repository
        .patch(
            &PersonalizationPolicyScope::Global,
            Some(0),
            PersonalizationPolicyPatch {
                about_user: Some("x".repeat(INSTRUCTION_FIELD_MAX_CHARS + 1)),
                ..PersonalizationPolicyPatch::default()
            },
            now(),
        )
        .expect_err("an oversized field must be rejected natively");
    assert!(matches!(error, PersonalizationApplicationError::Domain(_)));

    let stored = fixture
        .repository
        .load(&PersonalizationPolicyScope::Global)
        .expect("load")
        .expect("row");
    assert_eq!(
        stored.revision(),
        0,
        "a rejected write must leave the revision untouched"
    );
}

#[test]
fn expecting_a_revision_from_a_row_that_does_not_exist_is_not_found() {
    // Creating the row here would silently discard whatever the caller was editing against.
    let fixture = fixture("missing");
    let error = fixture
        .repository
        .patch(
            &agent_scope(),
            Some(3),
            PersonalizationPolicyPatch::default(),
            now(),
        )
        .expect_err("no row to expect a revision from");
    assert!(matches!(error, PersonalizationApplicationError::NotFound));
}

#[test]
fn loading_layers_returns_exactly_the_scopes_that_apply() {
    let fixture = fixture("layers");
    fixture.repository.seed_default_global(now()).expect("seed");

    let workspace_scope = PersonalizationPolicyScope::Workspace {
        workspace_key: workspace(),
    };
    let seat_scope = PersonalizationPolicyScope::WorkspaceAgent {
        workspace_key: workspace(),
        agent_id: agent(),
    };
    let other_agent_scope = PersonalizationPolicyScope::Agent {
        agent_id: AgentId::parse("codex-cli").expect("agent"),
    };
    for scope in [
        &agent_scope(),
        &workspace_scope,
        &seat_scope,
        &other_agent_scope,
    ] {
        fixture
            .repository
            .patch(scope, None, PersonalizationPolicyPatch::default(), now())
            .expect("create scope row");
    }

    let layers = fixture
        .repository
        .load_layers(&agent(), Some(&workspace()))
        .expect("layers");
    assert!(layers.global.is_some());
    assert_eq!(
        layers.agent.as_ref().map(|row| row.scope()),
        Some(&agent_scope())
    );
    assert_eq!(
        layers.workspace.as_ref().map(|row| row.scope()),
        Some(&workspace_scope)
    );
    assert_eq!(
        layers.workspace_agent.as_ref().map(|row| row.scope()),
        Some(&seat_scope)
    );
    assert!(
        layers.session_override.is_none(),
        "a session override is not a durable row"
    );
}

#[test]
fn loading_layers_without_a_workspace_omits_both_workspace_scopes() {
    let fixture = fixture("no-workspace");
    fixture.repository.seed_default_global(now()).expect("seed");
    let workspace_scope = PersonalizationPolicyScope::Workspace {
        workspace_key: workspace(),
    };
    fixture
        .repository
        .patch(
            &workspace_scope,
            None,
            PersonalizationPolicyPatch::default(),
            now(),
        )
        .expect("workspace row");

    let layers = fixture
        .repository
        .load_layers(&agent(), None)
        .expect("layers");
    assert!(layers.global.is_some());
    assert!(layers.workspace.is_none());
    assert!(layers.workspace_agent.is_none());
}

#[test]
fn another_agents_override_never_leaks_into_these_layers() {
    let fixture = fixture("other-agent");
    fixture.repository.seed_default_global(now()).expect("seed");
    fixture
        .repository
        .patch(
            &PersonalizationPolicyScope::Agent {
                agent_id: AgentId::parse("codex-cli").expect("agent"),
            },
            None,
            PersonalizationPolicyPatch {
                memory_read_mode: Some(PolicyToggle::Disabled),
                ..PersonalizationPolicyPatch::default()
            },
            now(),
        )
        .expect("other agent row");

    let layers = fixture
        .repository
        .load_layers(&agent(), None)
        .expect("layers");
    assert!(
        layers.agent.is_none(),
        "one seat must not resolve another seat's Agent policy"
    );
}

#[test]
fn policy_rows_survive_a_reopen() {
    let directory = TempDir::with_prefix("personalization-policy-reopen-").expect("directory");
    {
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let repository = SqlitePolicyRepository::new(database);
        repository.seed_default_global(now()).expect("seed");
        repository
            .patch(
                &PersonalizationPolicyScope::Global,
                Some(0),
                PersonalizationPolicyPatch {
                    style_rules: Some("be terse".to_string()),
                    ..PersonalizationPolicyPatch::default()
                },
                now(),
            )
            .expect("save");
    }

    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("reopen");
    let repository = SqlitePolicyRepository::new(database);
    let stored = repository
        .load(&PersonalizationPolicyScope::Global)
        .expect("load")
        .expect("row");
    assert_eq!(stored.style_rules(), "be terse");
    assert_eq!(stored.revision(), 1);
}

#[test]
fn deleting_a_scope_removes_only_that_row() {
    let fixture = fixture("delete");
    fixture.repository.seed_default_global(now()).expect("seed");
    fixture
        .repository
        .patch(
            &agent_scope(),
            None,
            PersonalizationPolicyPatch::default(),
            now(),
        )
        .expect("agent row");

    assert!(fixture.repository.delete(&agent_scope()).expect("delete"));
    assert!(!fixture
        .repository
        .delete(&agent_scope())
        .expect("second delete is a no-op"));
    assert!(fixture
        .repository
        .load(&PersonalizationPolicyScope::Global)
        .expect("load")
        .is_some());
}

#[test]
fn a_resolution_bundle_reports_absent_rather_than_omitting_a_key() {
    // The distinction that decides whether a result may be reused: proving a workspace has no
    // override is what lets a later resolution skip re-reading it, while a read that simply
    // returned nothing must never be cached as "no override".
    let fixture = fixture("bundle-absent");
    fixture
        .repository
        .patch(
            &PersonalizationPolicyScope::Global,
            None,
            PersonalizationPolicyPatch {
                about_user: Some("stored".to_string()),
                ..PersonalizationPolicyPatch::default()
            },
            now(),
        )
        .expect("seed global");

    let scopes = vec![
        PersonalizationPolicyScope::Global,
        PersonalizationPolicyScope::Agent {
            agent_id: AgentId::parse("onepiece").expect("agent"),
        },
    ];
    let bundle = fixture
        .repository
        .load_resolution_bundle(&scopes)
        .expect("bundle");

    assert_eq!(bundle.layers.len(), 2, "every requested key has an entry");
    assert!(matches!(
        bundle.state(&scopes[0]),
        Some(PolicyLayerState::Present(_))
    ));
    assert!(matches!(
        bundle.state(&scopes[1]),
        Some(PolicyLayerState::Absent)
    ));
    // A key that was never asked for is a different state again: not in the bundle at all.
    assert!(bundle
        .state(&PersonalizationPolicyScope::Workspace {
            workspace_key: WorkspaceKey::parse("ws_never_asked").expect("workspace"),
        })
        .is_none());
}

#[test]
fn an_empty_scope_list_reads_nothing_rather_than_everything() {
    let fixture = fixture("bundle-empty");

    let bundle = fixture
        .repository
        .load_resolution_bundle(&[])
        .expect("bundle");

    assert!(bundle.layers.is_empty());
}
