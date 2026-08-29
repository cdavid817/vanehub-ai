use super::*;
use rusqlite::{params, Connection};

const TABLES: &[&str] = &[
    "evolution_curator_candidates",
    "evolution_curator_candidate_sources",
    "evolution_curator_intake_receipts",
    "evolution_curator_drafts",
    "evolution_curator_draft_assessments",
    "evolution_curator_previews",
    "evolution_curator_decisions",
    "evolution_curator_events",
    "evolution_curator_applications",
    "evolution_curator_outbox",
    "evolution_curator_system_policy_authorizations",
    "evolution_curator_rollback_candidates",
    "evolution_curator_policy",
    "evolution_curator_notification_receipts",
];

fn connection_with_assessment_boundary() -> Connection {
    let connection = Connection::open_in_memory().expect("database");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("foreign keys");
    connection
        .execute_batch("CREATE TABLE evolution_assessment_attempts (attempt_id TEXT PRIMARY KEY);")
        .expect("assessment boundary fixture");
    connection
}

#[test]
fn schema_is_idempotent_and_creates_normalized_curator_tables() {
    let connection = connection_with_assessment_boundary();
    apply_schema(&connection).expect("first schema application");
    apply_schema(&connection).expect("idempotent schema application");

    for table in TABLES {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                params![table],
                |row| row.get(0),
            )
            .expect("table lookup");
        assert_eq!(count, 1, "missing table {table}");
    }
}

#[test]
fn migration_history_includes_curator_foundation() {
    let connection = Connection::open_in_memory().expect("database");
    crate::platform::database::migrate(&connection).expect("migrate");

    let name: String = connection
        .query_row(
            "SELECT name FROM schema_migrations WHERE version = 90",
            [],
            |row| row.get(0),
        )
        .expect("curator migration");
    assert_eq!(name, "skill-evolution-curator-foundation");
}

#[test]
fn migration_reapplies_after_only_its_history_marker_is_rolled_back() {
    let connection = Connection::open_in_memory().expect("database");
    crate::platform::database::migrate(&connection).expect("initial migrate");
    connection
        .execute("DELETE FROM schema_migrations WHERE version = 90", [])
        .expect("roll back marker");

    crate::platform::database::migrate(&connection).expect("idempotent reapply");
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = 90
             AND name = 'skill-evolution-curator-foundation'",
            [],
            |row| row.get(0),
        )
        .expect("migration marker");
    assert_eq!(count, 1);
}
