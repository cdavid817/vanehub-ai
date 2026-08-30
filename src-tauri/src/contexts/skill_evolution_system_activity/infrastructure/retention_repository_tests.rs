use rusqlite::{params, Connection};

use super::*;
use crate::contexts::skill_evolution_system_activity::domain::*;

const NOW_MS: i64 = 40 * 86_400_000;

#[test]
fn detail_retention_is_transactional_preserves_summaries_and_mandatory_events() {
    let connection = fixture();
    insert_event(
        &connection,
        "routine-old",
        "run_completed",
        "info",
        "none",
        1,
        1,
    );
    insert_event(
        &connection,
        "security-old",
        "run_failed",
        "warning",
        "security",
        2,
        1,
    );
    insert_event(
        &connection,
        "routine-new",
        "run_completed",
        "info",
        "none",
        3,
        NOW_MS,
    );
    connection
        .execute(
            "INSERT INTO evolution_activity_dashboard_state
         (scope_kind,canonical_scope_id,generation_id,materialization_kind,state_json,
          last_event_id,updated_at_ms) VALUES ('workspace','workspace-1','generation-1',
          'current_runs','{}','routine-new',?1)",
            [NOW_MS],
        )
        .expect("dashboard");

    let repository = SqliteActivityProjectionRepository::new(&connection);
    let report = repository
        .apply_detail_retention("session-1", NOW_MS)
        .expect("retention");
    assert_eq!(report.removed_items, 1);
    assert_eq!(report.preserved_mandatory_items, 1);
    assert_eq!(count(&connection, "evolution_activity_items"), 2);
    assert_eq!(count(&connection, "evolution_system_activity_sessions"), 1);
    assert_eq!(count(&connection, "evolution_activity_dashboard_state"), 1);
    assert_eq!(count(&connection, "evolution_activity_read_state"), 1);
    assert_eq!(session_unread(&connection), 2);
    assert_eq!(identity_count(&connection, "routine-old"), 0);
}

#[test]
fn source_purge_removes_drilldown_preserves_safe_outcomes_and_never_touches_source() {
    let connection = fixture();
    connection
        .execute_batch(
            "CREATE TABLE authoritative_evidence(source_id TEXT PRIMARY KEY, body TEXT NOT NULL);
         INSERT INTO authoritative_evidence VALUES ('evidence-1','sensitive-authoritative-body');",
        )
        .expect("source sentinel");
    insert_event(
        &connection,
        "evidence-detail",
        "evidence_ready",
        "info",
        "none",
        1,
        1,
    );
    insert_event(
        &connection,
        "applied-outcome",
        "overlay_applied",
        "warning",
        "review",
        2,
        2,
    );
    insert_event(
        &connection,
        "purge-event",
        "source_purged",
        "warning",
        "integrity",
        3,
        3,
    );
    for event_id in ["evidence-detail", "applied-outcome", "purge-event"] {
        connection
            .execute(
                "INSERT INTO evolution_activity_safe_identities
             (event_id,identity_kind,identity_value,normalized_value)
             VALUES (?1,'evidence','evidence-1','evidence-1')",
                [event_id],
            )
            .expect("evidence identity");
    }
    connection
        .execute(
            "INSERT INTO evolution_activity_safe_identities
         (event_id,identity_kind,identity_value,normalized_value)
         VALUES ('applied-outcome','skill','skill-1','skill-1')",
            [],
        )
        .expect("skill identity");

    let repository = SqliteActivityProjectionRepository::new(&connection);
    let report = repository
        .apply_source_purge(EvolutionSourceDomain::Evidence, "evidence-1", NOW_MS)
        .expect("source purge");
    assert_eq!(report.removed_detail_items, 1);
    assert_eq!(report.preserved_tombstones, 2);
    assert_eq!(count(&connection, "evolution_activity_purge_tombstones"), 3);
    assert_eq!(count(&connection, "evolution_activity_items"), 2);
    assert_eq!(identity_count(&connection, "applied-outcome"), 1);
    assert_eq!(
        connection
            .query_row(
                "SELECT body FROM authoritative_evidence WHERE source_id='evidence-1'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("source remains"),
        "sensitive-authoritative-body"
    );
}

fn fixture() -> Connection {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    connection
        .execute_batch(
            "INSERT INTO evolution_system_activity_sessions
         (session_id,schema_version,activity_kind,scope_kind,canonical_scope_id,
          active_generation_id,last_sequence,unread_count,attention_kind,preference_revision,
          created_at_ms,first_activity_at_ms,last_activity_at_ms,last_projected_at_ms)
         VALUES ('session-1',1,'skill_evolution','workspace','workspace-1','generation-1',
                 3,3,'security',1,1,1,1,1);
         INSERT INTO evolution_activity_preferences
         (scope_kind,canonical_scope_id,detail_retention_days,updated_at_ms)
         VALUES ('workspace','workspace-1',30,1);
         INSERT INTO evolution_activity_read_state
         (session_id,user_id,highest_read_sequence,last_seen_at_ms,revision)
         VALUES ('session-1','local',0,1,1);",
        )
        .expect("session fixture");
    connection
}

fn insert_event(
    connection: &Connection,
    event_id: &str,
    event_code: &str,
    severity: &str,
    attention: &str,
    sequence: i64,
    committed_at_ms: i64,
) {
    connection
        .execute(
            "INSERT INTO evolution_activity_envelopes
         (event_id,schema_version,event_code,source_domain,source_id,source_revision,
          source_sequence,scope_kind,canonical_scope_id,occurred_at_ms,committed_at_ms,
          severity,status,attention_kind,envelope_json,payload_json,projection_version,content_hash)
         VALUES (?1,1,?2,'evidence','evidence-1',?1,?3,'workspace','workspace-1',?4,?4,
                 ?5,'succeeded',?6,'{}','{}',1,?7)",
            params![
                event_id,
                event_code,
                sequence,
                committed_at_ms,
                severity,
                attention,
                format!("hash:{event_id}")
            ],
        )
        .expect("envelope");
    connection
        .execute(
            "INSERT INTO evolution_activity_items
         (item_id,session_id,generation_id,sequence,event_id,created_at_ms)
         VALUES (?1,'session-1','generation-1',?2,?3,?4)",
            params![
                format!("item:{event_id}"),
                sequence,
                event_id,
                committed_at_ms
            ],
        )
        .expect("item");
    connection
        .execute(
            "INSERT INTO evolution_activity_safe_identities
         (event_id,identity_kind,identity_value,normalized_value)
         VALUES (?1,'run',?1,?1)",
            [event_id],
        )
        .expect("identity");
}

fn count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count")
}

fn identity_count(connection: &Connection, event_id: &str) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM evolution_activity_safe_identities WHERE event_id=?1",
            [event_id],
            |row| row.get(0),
        )
        .expect("identity count")
}

fn session_unread(connection: &Connection) -> i64 {
    connection.query_row(
        "SELECT unread_count FROM evolution_system_activity_sessions WHERE session_id='session-1'",
        [],
        |row| row.get(0),
    ).expect("unread")
}
