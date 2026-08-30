use super::*;
use rusqlite::{params, Connection};

const TABLES: &[&str] = &[
    "evolution_system_activity_sessions",
    "evolution_activity_envelopes",
    "evolution_activity_safe_identities",
    "evolution_activity_purge_tombstones",
    "evolution_activity_notification_requests",
    "evolution_activity_items",
    "evolution_activity_source_receipts",
    "evolution_activity_target_receipts",
    "evolution_activity_domain_cursors",
    "evolution_activity_dashboard_state",
    "evolution_activity_read_state",
    "evolution_activity_preferences",
    "evolution_activity_digest_buckets",
    "evolution_activity_projection_leases",
    "evolution_activity_rebuilds",
    "evolution_activity_rebuild_checkpoints",
    "evolution_activity_exports",
];

#[test]
fn schema_is_idempotent_and_creates_all_projection_tables() {
    let connection = Connection::open_in_memory().expect("database");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("foreign keys");
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
fn session_scope_is_collision_safe_and_preferences_are_optimistic() {
    let connection = Connection::open_in_memory().expect("database");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("foreign keys");
    apply_schema(&connection).expect("schema");
    connection.execute(
        "INSERT INTO evolution_system_activity_sessions (
            session_id, schema_version, activity_kind, scope_kind, canonical_scope_id,
            active_generation_id, created_at_ms, first_activity_at_ms, last_activity_at_ms,
            last_projected_at_ms
         ) VALUES (?1, 1, 'skill_evolution', 'workspace', 'workspace:one', 'generation-1', 1, 1, 1, 1)",
        ["session-one"],
    ).expect("session");
    assert!(connection.execute(
        "INSERT INTO evolution_system_activity_sessions (
            session_id, schema_version, activity_kind, scope_kind, canonical_scope_id,
            active_generation_id, created_at_ms, first_activity_at_ms, last_activity_at_ms,
            last_projected_at_ms
         ) VALUES (?1, 1, 'skill_evolution', 'workspace', 'workspace:one', 'generation-2', 2, 2, 2, 2)",
        ["session-collision"],
    ).is_err());

    connection
        .execute(
            "INSERT INTO evolution_activity_preferences (
            scope_kind, canonical_scope_id, updated_at_ms
         ) VALUES ('workspace', 'workspace:one', 1)",
            [],
        )
        .expect("preferences");
    assert_eq!(
        connection
            .execute(
                "UPDATE evolution_activity_preferences SET visible = 0, revision = revision + 1
         WHERE scope_kind = 'workspace' AND canonical_scope_id = 'workspace:one' AND revision = 1",
                [],
            )
            .expect("current revision"),
        1
    );
    assert_eq!(
        connection
            .execute(
                "UPDATE evolution_activity_preferences SET visible = 1, revision = revision + 1
         WHERE scope_kind = 'workspace' AND canonical_scope_id = 'workspace:one' AND revision = 1",
                [],
            )
            .expect("stale revision"),
        0
    );
}

#[test]
fn persisted_timeline_items_cannot_be_rewritten() {
    let connection = Connection::open_in_memory().expect("database");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("foreign keys");
    apply_schema(&connection).expect("schema");
    connection
        .execute_batch(
            "INSERT INTO evolution_system_activity_sessions (
            session_id, schema_version, activity_kind, scope_kind, canonical_scope_id,
            active_generation_id, created_at_ms, first_activity_at_ms, last_activity_at_ms,
            last_projected_at_ms
         ) VALUES ('session-one', 1, 'skill_evolution', 'global', 'global',
                   'generation-1', 1, 1, 1, 1);
         INSERT INTO evolution_activity_envelopes (
            event_id, schema_version, event_code, source_domain, source_id, source_revision,
            source_sequence, scope_kind, canonical_scope_id, occurred_at_ms, committed_at_ms,
            severity, status, attention_kind, envelope_json, projection_version, content_hash
         ) VALUES ('event-one', 1, 'run_completed', 'orchestration', 'run-one', 'revision-one',
                   1, 'global', 'global', 1, 1, 'info', 'succeeded', 'none', '{}', 1, 'sha256:one');
         INSERT INTO evolution_activity_items (
            item_id, session_id, generation_id, sequence, event_id, created_at_ms
         ) VALUES ('item-one', 'session-one', 'generation-1', 1, 'event-one', 1);",
        )
        .expect("projection fixture");

    assert!(connection
        .execute(
            "UPDATE evolution_activity_items SET sequence = 2 WHERE item_id = 'item-one'",
            [],
        )
        .is_err());
}

#[test]
fn source_outboxes_commit_atomically_and_reject_non_owned_domains() {
    let connection = Connection::open_in_memory().expect("database");
    crate::platform::database::migrate(&connection).expect("all source schemas");
    connection
        .execute_batch(
            "INSERT INTO evolution_run_requests (
               request_id,schema_version,workspace_id,actor,status,trigger_counters_json,
               follow_up,not_before_ms,revision,created_at_ms,updated_at_ms
             ) VALUES ('request-one',1,'workspace-one','system','completed','{}',0,0,0,1,1);
             INSERT INTO evolution_runs (
               run_id,schema_version,request_id,workspace_id,status,current_stage,
               policy_witness_hash,budget_json,usage_json,revision,created_at_ms,updated_at_ms
             ) VALUES ('run-one',1,'request-one','workspace-one','requested',NULL,
                       'sha256:policy','{}','{}',0,1,1);",
        )
        .expect("source commit");
    assert_eq!(
        outbox_count(&connection, "evolution_orchestration_activity_outbox"),
        1
    );

    let transaction = connection.unchecked_transaction().expect("transaction");
    transaction
        .execute(
            "UPDATE evolution_runs SET status='running',revision=1,updated_at_ms=2
             WHERE run_id='run-one'",
            [],
        )
        .expect("transactional source update");
    assert_eq!(
        outbox_count(&transaction, "evolution_orchestration_activity_outbox"),
        2
    );
    transaction.rollback().expect("rollback");
    assert_eq!(
        outbox_count(&connection, "evolution_orchestration_activity_outbox"),
        1
    );

    assert!(connection
        .execute(
            "UPDATE evolution_orchestration_activity_outbox SET event_kind='rewritten'",
            [],
        )
        .is_err());
    assert!(connection
        .execute(
            "INSERT INTO evolution_evidence_activity_outbox
             (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
              committed_at_ms,source_integrity_witness)
             VALUES ('invalid-domain','curator','candidate','candidate-one','1','created',1,'hash')",
            [],
        )
        .is_err());
}

fn outbox_count(connection: &Connection, table: &str) -> i64 {
    let query = format!("SELECT COUNT(*) FROM {table}");
    connection
        .query_row(&query, [], |row| row.get(0))
        .expect("outbox count")
}
