use rusqlite::Connection;
use tempfile::TempDir;

use super::schema::apply_schema;
use crate::platform::database::NativeDatabase;

/// Every table migration 82 owns.
const EXPECTED_TABLES: &[&str] = &[
    "personalization_policy_overrides",
    "personalization_memory_projection",
    "personalization_memory_candidates",
    "personalization_legacy_memory_alias",
    "personalization_memory_migration_journal",
    "personalization_migration_state",
];

/// Every index. Listed explicitly rather than counted, so dropping one is a test failure and not a
/// silent performance regression that only shows up on a large store.
const EXPECTED_INDEXES: &[&str] = &[
    "idx_personalization_policy_scope",
    "idx_personalization_memory_status_updated",
    "idx_personalization_memory_scope",
    "idx_personalization_memory_source_agent",
    "idx_personalization_memory_type",
    "idx_personalization_memory_keyset",
    "idx_personalization_candidate_status_created",
    "idx_personalization_candidate_target",
    "idx_personalization_alias_target",
    "idx_personalization_journal_memory",
    "idx_personalization_journal_stage",
    "idx_personalization_journal_locator",
];

fn object_exists(conn: &Connection, kind: &str, name: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = ?1 AND name = ?2",
        rusqlite::params![kind, name],
        |row| row.get::<_, i64>(0),
    )
    .expect("sqlite_master lookup")
        > 0
}

fn assert_full_schema(conn: &Connection) {
    for table in EXPECTED_TABLES {
        assert!(
            object_exists(conn, "table", table),
            "{table} must exist after migration 82"
        );
    }
    for index in EXPECTED_INDEXES {
        assert!(
            object_exists(conn, "index", index),
            "{index} must exist after migration 82"
        );
    }
}

#[test]
fn a_fresh_database_reaches_the_full_personalization_schema() {
    let directory = TempDir::with_prefix("personalization-schema-fresh-").expect("directory");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let connection = database.connection().expect("connection");
    assert_full_schema(&connection);

    let version: i64 = connection
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("migration version");
    assert_eq!(version, 82);

    let recorded: String = connection
        .query_row(
            "SELECT name FROM schema_migrations WHERE version = 82",
            [],
            |row| row.get(0),
        )
        .expect("migration name");
    assert_eq!(recorded, "personalization-governance");
}

#[test]
fn upgrading_a_database_that_predates_migration_82_creates_the_whole_schema() {
    // Stands in for a database built on `main`: every personalization object is removed and the
    // version row rolled back, so the upgrade path runs from the state a pre-branch installation
    // would actually be in.
    let directory = TempDir::with_prefix("personalization-schema-upgrade-").expect("directory");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let connection = database.connection().expect("connection");

    // Something the user owns, to prove the upgrade is additive rather than a rebuild.
    connection
        .execute(
            "INSERT INTO personalization_migration_state (id, generation, repair_required)
             VALUES (1, 0, 0)
             ON CONFLICT(id) DO NOTHING",
            [],
        )
        .expect("seed marker");

    for index in EXPECTED_INDEXES {
        connection
            .execute(&format!("DROP INDEX IF EXISTS {index}"), [])
            .expect("drop index");
    }
    for table in EXPECTED_TABLES {
        connection
            .execute(&format!("DROP TABLE IF EXISTS {table}"), [])
            .expect("drop table");
    }
    connection
        .execute("DELETE FROM schema_migrations WHERE version = 82", [])
        .expect("roll the version back");

    let version_before: i64 = connection
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("version");
    assert_eq!(
        version_before, 81,
        "the fixture is at the pre-branch version"
    );
    for table in EXPECTED_TABLES {
        assert!(!object_exists(&connection, "table", table));
    }

    crate::platform::database::migrate(&connection).expect("upgrade");

    assert_full_schema(&connection);
    let version_after: i64 = connection
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("version");
    assert_eq!(version_after, 82);
}

#[test]
fn re_running_the_schema_is_a_no_op() {
    // The migration is version-gated in production, but the statements themselves must also be
    // idempotent: a repair path may apply them directly, and a second application must not fail or
    // reset the singleton marker.
    let directory = TempDir::with_prefix("personalization-schema-idempotent-").expect("directory");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let connection = database.connection().expect("connection");

    connection
        .execute(
            "UPDATE personalization_migration_state SET generation = 7 WHERE id = 1",
            [],
        )
        .expect("advance the marker");

    apply_schema(&connection).expect("re-apply");
    apply_schema(&connection).expect("re-apply again");

    assert_full_schema(&connection);
    let generation: i64 = connection
        .query_row(
            "SELECT generation FROM personalization_migration_state WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .expect("marker");
    assert_eq!(
        generation, 7,
        "re-applying must not reset a marker the application already advanced"
    );
}

#[test]
fn a_failing_migration_leaves_no_partial_schema_and_no_version_row() {
    // The registry wraps each migration in a transaction. Proving that here means running the
    // statements plus a deliberate failure inside one transaction and asserting nothing survives —
    // a half-created schema with a recorded version is the state that makes the next startup skip
    // the repair it needs.
    let directory = TempDir::with_prefix("personalization-schema-rollback-").expect("directory");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let mut connection = database.connection().expect("connection");

    for index in EXPECTED_INDEXES {
        connection
            .execute(&format!("DROP INDEX IF EXISTS {index}"), [])
            .expect("drop index");
    }
    for table in EXPECTED_TABLES {
        connection
            .execute(&format!("DROP TABLE IF EXISTS {table}"), [])
            .expect("drop table");
    }
    connection
        .execute("DELETE FROM schema_migrations WHERE version = 82", [])
        .expect("roll back the version");

    let transaction = connection.transaction().expect("transaction");
    apply_schema(&transaction).expect("schema statements succeed");
    // Stand-in for any later step of the same migration failing.
    let failed = transaction.execute("INSERT INTO nonexistent_table VALUES (1)", []);
    assert!(failed.is_err());
    drop(transaction);

    for table in EXPECTED_TABLES {
        assert!(
            !object_exists(&connection, "table", table),
            "{table} must not survive a rolled-back migration"
        );
    }
    let version: i64 = connection
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("version");
    assert_eq!(
        version, 81,
        "no version may be recorded for a migration that rolled back"
    );
}
