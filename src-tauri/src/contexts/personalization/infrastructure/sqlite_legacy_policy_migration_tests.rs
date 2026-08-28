use chrono::{TimeZone, Utc};
use tempfile::TempDir;

use super::sqlite_legacy_policy_migration::SqliteLegacyPolicyMigration;
use super::sqlite_migration_state::SqliteMigrationState;
use super::sqlite_policy_repository::SqlitePolicyRepository;
use crate::contexts::personalization::application::{
    map_legacy_settings, LegacyPersonalizationSettings, LegacyPolicyMigrationPort,
    MigrationStatePort, PersonalizationApplicationError, PolicyRepository,
};
use crate::contexts::personalization::domain::{
    AgentId, InstructionMergeMode, PersonalizationPolicyScope, PolicyToggle,
};
use crate::platform::database::NativeDatabase;

struct Fixture {
    _directory: TempDir,
    migration: SqliteLegacyPolicyMigration,
    policies: SqlitePolicyRepository,
    state: SqliteMigrationState,
}

fn fixture(label: &str) -> Fixture {
    let directory =
        TempDir::with_prefix(format!("legacy-policy-{label}-")).expect("temporary directory");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    Fixture {
        _directory: directory,
        migration: SqliteLegacyPolicyMigration::new(database.clone()),
        policies: SqlitePolicyRepository::new(database.clone()),
        state: SqliteMigrationState::new(database),
    }
}

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 24, 9, 0, 0).unwrap()
}

fn onepiece_scope() -> PersonalizationPolicyScope {
    PersonalizationPolicyScope::Agent {
        agent_id: AgentId::parse("onepiece").expect("agent"),
    }
}

fn legacy() -> LegacyPersonalizationSettings {
    LegacyPersonalizationSettings {
        about_user: Some("I write Rust".to_string()),
        style_rules: Some("Be terse".to_string()),
        custom_instructions_enabled: Some(true),
        memory_enabled: Some(false),
        tool_assisted_extraction_enabled: Some(true),
    }
}

#[test]
fn a_fresh_database_reports_migration_incomplete() {
    let fixture = fixture("incomplete");
    assert!(!fixture.migration.is_complete().expect("read marker"));
    assert!(fixture
        .policies
        .load(&PersonalizationPolicyScope::Global)
        .expect("load")
        .is_none());
}

#[test]
fn committing_writes_the_rows_and_the_marker_together() {
    let fixture = fixture("commit");
    let migrated = map_legacy_settings(&legacy()).expect("map");

    assert!(fixture.migration.commit(&migrated, now()).expect("commit"));
    assert!(fixture.migration.is_complete().expect("marker"));

    let global = fixture
        .policies
        .load(&PersonalizationPolicyScope::Global)
        .expect("load")
        .expect("the global row exists");
    assert_eq!(global.about_user(), "I write Rust");
    assert_eq!(global.memory_read_mode(), PolicyToggle::Disabled);
    assert_eq!(
        global.instruction_merge_mode(),
        InstructionMergeMode::Append
    );

    let override_record = fixture
        .policies
        .load(&onepiece_scope())
        .expect("load")
        .expect("the OnePiece override exists");
    assert_eq!(
        override_record.automatic_extraction_mode(),
        PolicyToggle::Enabled
    );

    let state = fixture.state.load().expect("state");
    assert_eq!(state.generation, 1);
    assert!(state.completed_at.is_some());
    assert!(state.started_at.is_some());
}

#[test]
fn a_repeated_startup_is_a_no_op_and_does_not_reset_later_edits() {
    // The rewrite this prevents: a second migration resetting a revision the user has since
    // advanced, which would make their next expected-revision save conflict for no reason.
    let fixture = fixture("repeat");
    let migrated = map_legacy_settings(&legacy()).expect("map");
    assert!(fixture.migration.commit(&migrated, now()).expect("first"));

    fixture
        .policies
        .patch(
            &PersonalizationPolicyScope::Global,
            Some(0),
            crate::contexts::personalization::domain::PersonalizationPolicyPatch {
                about_user: Some("edited after migration".to_string()),
                ..Default::default()
            },
            now(),
        )
        .expect("user edit");

    assert!(
        !fixture.migration.commit(&migrated, now()).expect("second"),
        "a completed migration reports no work rather than redoing it"
    );

    let global = fixture
        .policies
        .load(&PersonalizationPolicyScope::Global)
        .expect("load")
        .expect("row");
    assert_eq!(global.about_user(), "edited after migration");
    assert_eq!(global.revision(), 1, "the user's revision survived");
}

#[test]
fn a_failure_partway_through_leaves_no_rows_and_no_marker() {
    // Rollback evidence: the marker row is removed so advancing it fails after the policy rows have
    // already been inserted inside the transaction. Nothing may survive.
    let fixture = fixture("rollback");
    let connection = fixture
        .policies
        .raw_connection_for_tests()
        .expect("connection");
    connection
        .execute(
            "DELETE FROM personalization_migration_state WHERE id = 1",
            [],
        )
        .expect("remove the marker row");
    drop(connection);

    let migrated = map_legacy_settings(&legacy()).expect("map");
    let error = fixture
        .migration
        .commit(&migrated, now())
        .expect_err("the marker cannot be advanced");
    assert!(matches!(error, PersonalizationApplicationError::Storage(_)));

    assert!(
        fixture
            .policies
            .load(&PersonalizationPolicyScope::Global)
            .expect("load")
            .is_none(),
        "the global row must not survive a failed migration"
    );
    assert!(
        fixture
            .policies
            .load(&onepiece_scope())
            .expect("load")
            .is_none(),
        "the override must not survive either"
    );
    assert!(
        fixture.policies.list_all().expect("list").is_empty(),
        "the transaction rolled back completely"
    );
}

#[test]
fn a_failed_migration_can_be_retried_once_the_cause_is_repaired() {
    let fixture = fixture("retry");
    let connection = fixture
        .policies
        .raw_connection_for_tests()
        .expect("connection");
    connection
        .execute(
            "DELETE FROM personalization_migration_state WHERE id = 1",
            [],
        )
        .expect("remove the marker row");
    drop(connection);

    let migrated = map_legacy_settings(&legacy()).expect("map");
    assert!(fixture.migration.commit(&migrated, now()).is_err());

    let connection = fixture
        .policies
        .raw_connection_for_tests()
        .expect("connection");
    connection
        .execute(
            "INSERT INTO personalization_migration_state (id, generation, repair_required)
             VALUES (1, 0, 0)",
            [],
        )
        .expect("restore the marker row");
    drop(connection);

    assert!(fixture.migration.commit(&migrated, now()).expect("retry"));
    assert!(fixture.migration.is_complete().expect("marker"));
    assert_eq!(fixture.policies.list_all().expect("list").len(), 2);
}

#[test]
fn a_migration_without_an_override_writes_exactly_one_row() {
    let fixture = fixture("single-row");
    let migrated = map_legacy_settings(&LegacyPersonalizationSettings {
        tool_assisted_extraction_enabled: None,
        ..legacy()
    })
    .expect("map");

    assert!(fixture.migration.commit(&migrated, now()).expect("commit"));
    assert_eq!(fixture.policies.list_all().expect("list").len(), 1);
    assert!(fixture
        .policies
        .load(&onepiece_scope())
        .expect("load")
        .is_none());
}

#[test]
fn migrated_rows_start_at_revision_zero_so_a_first_edit_expects_zero() {
    let fixture = fixture("revision");
    let migrated = map_legacy_settings(&legacy()).expect("map");
    fixture.migration.commit(&migrated, now()).expect("commit");

    let global = fixture
        .policies
        .load(&PersonalizationPolicyScope::Global)
        .expect("load")
        .expect("row");
    assert_eq!(global.revision(), 0);
}
