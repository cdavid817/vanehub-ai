use std::collections::BTreeMap;

use rusqlite::Connection;

use super::*;
use crate::contexts::skill_evolution_system_activity::{application::*, domain::*};

#[test]
fn source_receipt_replay_is_suppressed_and_checkpoint_is_atomic() {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let first = batch(event(1, "event-1", "revision-1", "hash-1"), 0);

    let inserted = repository
        .commit_projection_batch(&first)
        .expect("first projection");
    assert_eq!((inserted.inserted, inserted.replayed), (1, 0));

    let replay = batch(event(1, "event-1", "revision-1", "hash-1"), 1);
    let suppressed = repository
        .commit_projection_batch(&replay)
        .expect("source replay");
    assert_eq!((suppressed.inserted, suppressed.replayed), (0, 1));
    assert_eq!(count(&connection, "evolution_activity_envelopes"), 1);
    assert_eq!(count(&connection, "evolution_activity_source_receipts"), 1);
    assert_eq!(
        repository
            .cursor(EvolutionSourceDomain::Evidence)
            .expect("cursor query")
            .expect("cursor")
            .revision,
        2
    );
}

#[test]
fn receipt_collision_rolls_back_envelope_and_checkpoint() {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);
    repository
        .commit_projection_batch(&batch(event(1, "event-1", "revision-1", "hash-1"), 0))
        .expect("first projection");
    let collision = batch(event(2, "event-2", "revision-1", "hash-2"), 1);

    assert_eq!(
        repository.commit_projection_batch(&collision),
        Err(ActivityProjectionRepositoryError::ReceiptCollision)
    );
    assert_eq!(count(&connection, "evolution_activity_envelopes"), 1);
    assert_eq!(
        repository
            .cursor(EvolutionSourceDomain::Evidence)
            .expect("cursor query")
            .expect("cursor")
            .last_sequence,
        1
    );
}

#[test]
fn target_receipts_are_unique_and_only_failed_delivery_can_recover() {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);
    repository
        .commit_projection_batch(&batch(event(1, "event-1", "revision-1", "hash-1"), 0))
        .expect("envelope");
    let failed = target(ActivityDeliveryStatus::Failed, None);
    assert!(repository
        .record_target_receipt(&failed)
        .expect("failed receipt"));
    assert!(!repository
        .record_target_receipt(&failed)
        .expect("same failed receipt"));

    let delivered = target(ActivityDeliveryStatus::Delivered, Some(20));
    assert!(repository
        .record_target_receipt(&delivered)
        .expect("delivery recovery"));
    assert!(!repository
        .record_target_receipt(&delivered)
        .expect("delivered replay"));
    assert_eq!(
        repository.record_target_receipt(&target(ActivityDeliveryStatus::Suppressed, None)),
        Err(ActivityProjectionRepositoryError::ReceiptCollision)
    );
    assert_eq!(count(&connection, "evolution_activity_target_receipts"), 1);
}

#[test]
fn restart_continues_after_checkpoint_with_stable_same_timestamp_ordering() {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    SqliteActivityProjectionRepository::new(&connection)
        .commit_projection_batch(&batch(event(1, "event-b", "revision-1", "hash-1"), 0))
        .expect("first process lifetime");

    let restarted = SqliteActivityProjectionRepository::new(&connection);
    restarted
        .commit_projection_batch(&batch(event(2, "event-a", "revision-2", "hash-2"), 1))
        .expect("restart continuation");
    let ids = connection
        .prepare(
            "SELECT event_id FROM evolution_activity_envelopes
             ORDER BY committed_at_ms,source_sequence,event_id",
        )
        .expect("order query")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("ordered events")
        .collect::<Result<Vec<_>, _>>()
        .expect("event ids");

    assert_eq!(ids, vec!["event-b", "event-a"]);
    assert_eq!(
        restarted
            .cursor(EvolutionSourceDomain::Evidence)
            .expect("cursor query")
            .expect("cursor")
            .last_sequence,
        2
    );
}

fn batch(event: VerifiedProjectionEvent, expected_revision: u64) -> ActivityProjectionBatch {
    ActivityProjectionBatch {
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
    }
}

fn event(
    sequence: u64,
    event_id: &str,
    source_revision: &str,
    source_hash: &str,
) -> VerifiedProjectionEvent {
    VerifiedProjectionEvent {
        source_cursor: OpaqueDomainCursor::parse(format!("cursor:{sequence}")).expect("cursor"),
        source_sequence: sequence,
        source_integrity_hash: source_hash.into(),
        envelope: EvolutionActivityEnvelopeV1 {
            schema_version: ACTIVITY_SCHEMA_VERSION_V1,
            event_id: event_id.into(),
            event_code: ActivityEventCode::EvidenceReady,
            source_domain: "evidence".into(),
            source_id: "source-1".into(),
            source_revision: source_revision.into(),
            source_sequence: sequence,
            scope_kind: ActivityScopeKind::Workspace,
            canonical_scope_id: "workspace-1".into(),
            occurred_at_ms: 1,
            committed_at_ms: 2,
            severity: ActivitySeverity::Info,
            status: ActivityStatus::Succeeded,
            attention_kind: ActivityAttentionKind::None,
            safe_actor_kind: ActivityActorKind::System,
            safe_identities: Vec::new(),
            metrics: BTreeMap::new(),
            reason_codes: Vec::new(),
            navigation: None,
            supersedes_event_id: None,
            payload: None,
            projection_policy_version: 1,
            content_hash: String::new(),
        }
        .seal()
        .expect("envelope"),
    }
}

fn target(status: ActivityDeliveryStatus, delivered_at_ms: Option<i64>) -> ActivityTargetReceipt {
    ActivityTargetReceipt {
        event_id: "event-1".into(),
        target_kind: ActivityTargetKind::SystemTimeline,
        target_scope: "workspace-1".into(),
        status,
        delivered_at_ms,
    }
}

fn count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count")
}
