use super::apply_schema;
use rusqlite::Connection;

const TABLES: [&str; 18] = [
    "evolution_trigger_receipts",
    "evolution_run_requests",
    "evolution_run_request_trigger_links",
    "evolution_runs",
    "evolution_run_trigger_links",
    "evolution_run_stages",
    "evolution_run_items",
    "evolution_run_checkpoints",
    "evolution_orchestration_policy",
    "evolution_correction_authorizations",
    "evolution_deterministic_drafts",
    "evolution_auto_eligibility",
    "evolution_auto_rate_reservations",
    "evolution_auto_preflight_witnesses",
    "evolution_auto_breakers",
    "evolution_auto_applications",
    "evolution_auto_probations",
    "evolution_probation_observations",
];

#[test]
fn schema_creates_every_normalized_orchestration_table_idempotently() {
    let connection = Connection::open_in_memory().expect("database");
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .expect("foreign keys");
    apply_schema(&connection).expect("first migration");
    apply_schema(&connection).expect("idempotent migration");
    for table in TABLES {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |row| row.get(0),
            )
            .expect("table lookup");
        assert_eq!(count, 1, "missing table {table}");
    }
}

#[test]
fn schema_rejects_unknown_trigger_policy_and_application_actor_values() {
    let connection = Connection::open_in_memory().expect("database");
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .expect("foreign keys");
    apply_schema(&connection).expect("migration");
    assert!(connection.execute("INSERT INTO evolution_trigger_receipts VALUES ('receipt',1,'unknown','workspace','source','id',1,1,1,'[]','runtime_trigger',1)", []).is_err());
    assert!(connection.execute("INSERT INTO evolution_orchestration_policy VALUES ('workspace',1,'automatic','[]',NULL,'{}','{}',60000,900000,0,0,1,1)", []).is_err());
}
