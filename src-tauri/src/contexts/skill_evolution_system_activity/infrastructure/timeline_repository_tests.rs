use std::collections::BTreeMap;

use rusqlite::Connection;

use super::*;
use crate::contexts::skill_evolution_system_activity::{application::*, domain::*};

#[test]
fn first_timeline_delivery_lazily_creates_stable_session_and_preferences() {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);
    persist(&repository, event(1, "event-1", None), 0);
    assert_eq!(count(&connection, "evolution_system_activity_sessions"), 0);

    let first = repository
        .deliver_timeline("event-1", 20)
        .expect("first timeline delivery");
    assert!(first.session_created);
    assert!(first.item_created);
    assert_eq!(first.sequence, 1);
    assert_eq!(count(&connection, "evolution_activity_preferences"), 1);

    let replay = repository
        .deliver_timeline("event-1", 30)
        .expect("timeline replay");
    assert_eq!(replay.session_id, first.session_id);
    assert_eq!(replay.generation_id, first.generation_id);
    assert_eq!(replay.sequence, 1);
    assert!(!replay.session_created);
    assert!(!replay.item_created);
    assert_eq!(session_sequence(&connection), 1);
}

#[test]
fn detail_removal_preserves_session_identity_and_preferences() {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);
    persist(&repository, event(1, "event-1", None), 0);
    let delivered = repository
        .deliver_timeline("event-1", 20)
        .expect("timeline delivery");

    connection
        .execute("DELETE FROM evolution_activity_items", [])
        .expect("detail retention");

    assert_eq!(count(&connection, "evolution_system_activity_sessions"), 1);
    assert_eq!(count(&connection, "evolution_activity_preferences"), 1);
    assert_eq!(
        connection
            .query_row(
                "SELECT session_id FROM evolution_system_activity_sessions",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("session id"),
        delivered.session_id
    );
}

#[test]
fn timeline_sequences_are_monotonic_and_supersession_requires_visible_prior_item() {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);
    persist(&repository, event(1, "event-1", None), 0);
    repository
        .deliver_timeline("event-1", 20)
        .expect("first timeline delivery");
    persist(&repository, event(2, "event-2", Some("event-1")), 1);
    let second = repository
        .deliver_timeline("event-2", 30)
        .expect("supersession delivery");
    assert_eq!(second.sequence, 2);

    persist(&repository, event(3, "event-3", Some("missing-event")), 2);
    assert_eq!(
        repository.deliver_timeline("event-3", 40),
        Err(ActivityProjectionRepositoryError::InvalidInput)
    );
    assert_eq!(session_sequence(&connection), 2);
    assert_eq!(count(&connection, "evolution_activity_items"), 2);
}

fn persist(
    repository: &SqliteActivityProjectionRepository<'_>,
    event: VerifiedProjectionEvent,
    expected_revision: u64,
) {
    repository
        .commit_projection_batch(&ActivityProjectionBatch {
            checkpoint: ActivityDomainCheckpoint {
                source_domain: EvolutionSourceDomain::Evidence,
                opaque_cursor: event.source_cursor.clone(),
                last_sequence: event.source_sequence,
                last_source_hash: event.source_integrity_hash.clone(),
                retention_floor: None,
                pending_count: 0,
                oldest_pending_at_ms: None,
                last_success_at_ms: 10,
                expected_revision,
            },
            events: vec![event],
        })
        .expect("persist envelope");
}

fn event(
    sequence: u64,
    event_id: &str,
    supersedes_event_id: Option<&str>,
) -> VerifiedProjectionEvent {
    VerifiedProjectionEvent {
        source_cursor: OpaqueDomainCursor::parse(format!("cursor:{sequence}")).expect("cursor"),
        source_sequence: sequence,
        source_integrity_hash: format!("hash:{sequence}"),
        envelope: EvolutionActivityEnvelopeV1 {
            schema_version: ACTIVITY_SCHEMA_VERSION_V1,
            event_id: event_id.into(),
            event_code: ActivityEventCode::EvidenceReady,
            source_domain: "evidence".into(),
            source_id: format!("source-{sequence}"),
            source_revision: format!("revision-{sequence}"),
            source_sequence: sequence,
            scope_kind: ActivityScopeKind::Workspace,
            canonical_scope_id: "workspace-1".into(),
            occurred_at_ms: 1,
            committed_at_ms: 2,
            severity: ActivitySeverity::Info,
            status: ActivityStatus::Succeeded,
            attention_kind: ActivityAttentionKind::None,
            safe_actor_kind: ActivityActorKind::System,
            safe_identities: vec![SafeIdentity {
                kind: ActivitySafeIdentityKind::Workspace,
                value: "workspace-safe".into(),
            }],
            metrics: BTreeMap::new(),
            reason_codes: Vec::new(),
            navigation: None,
            supersedes_event_id: supersedes_event_id.map(str::to_owned),
            payload: None,
            projection_policy_version: 1,
            content_hash: String::new(),
        }
        .seal()
        .expect("envelope"),
    }
}

fn session_sequence(connection: &Connection) -> i64 {
    connection
        .query_row(
            "SELECT last_sequence FROM evolution_system_activity_sessions",
            [],
            |row| row.get(0),
        )
        .expect("session sequence")
}

fn count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count")
}
