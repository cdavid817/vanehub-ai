use std::collections::BTreeMap;

use rusqlite::Connection;

use super::*;
use crate::contexts::skill_evolution_system_activity::{application::*, domain::*};

#[test]
fn all_seven_dashboard_materializations_are_typed_and_idempotent() {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let codes = [
        ActivityEventCode::RunStarted,
        ActivityEventCode::AssessmentCompleted,
        ActivityEventCode::GenerationCompleted,
        ActivityEventCode::CuratorQueued,
        ActivityEventCode::OverlayApplied,
        ActivityEventCode::ProbationStarted,
        ActivityEventCode::BreakerOpened,
    ];
    for (index, code) in codes.into_iter().enumerate() {
        let sequence = u64::try_from(index + 1).expect("sequence");
        persist(&repository, event(sequence, code, 100), sequence - 1);
        let event_id = format!("event-{sequence}");
        let first = repository
            .materialize_dashboard(&event_id, 200)
            .expect("dashboard state");
        assert!(first.kind.is_some());
        assert!(first.materialized);
        assert!(
            !repository
                .materialize_dashboard(&event_id, 300)
                .expect("dashboard replay")
                .materialized
        );
    }
    assert_eq!(dashboard_count(&connection), 7);
}

#[test]
fn dashboard_uses_committed_time_sequence_and_event_id_for_stable_latest_state() {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);
    persist(&repository, event(1, ActivityEventCode::RunStarted, 100), 0);
    repository
        .materialize_dashboard("event-1", 200)
        .expect("first state");
    persist(
        &repository,
        event(2, ActivityEventCode::RunCompleted, 100),
        1,
    );
    assert!(
        repository
            .materialize_dashboard("event-2", 210)
            .expect("newer state")
            .materialized
    );
    persist(&repository, event(3, ActivityEventCode::RunFailed, 50), 2);
    assert!(
        !repository
            .materialize_dashboard("event-3", 220)
            .expect("stale state")
            .materialized
    );

    let latest = connection
        .query_row(
            "SELECT last_event_id,revision FROM evolution_activity_dashboard_state",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .expect("latest dashboard state");
    assert_eq!(latest, ("event-2".into(), 2));
}

#[test]
fn retention_and_recovery_events_do_not_fabricate_dashboard_state() {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);
    persist(
        &repository,
        event(1, ActivityEventCode::SourcePurged, 100),
        0,
    );

    let outcome = repository
        .materialize_dashboard("event-1", 200)
        .expect("non-dashboard event");
    assert_eq!(outcome.kind, None);
    assert!(!outcome.materialized);
    assert_eq!(dashboard_count(&connection), 0);
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
        .expect("persist event");
}

fn event(
    sequence: u64,
    event_code: ActivityEventCode,
    committed_at_ms: i64,
) -> VerifiedProjectionEvent {
    VerifiedProjectionEvent {
        source_cursor: OpaqueDomainCursor::parse(format!("cursor:{sequence}")).expect("cursor"),
        source_sequence: sequence,
        source_integrity_hash: format!("hash:{sequence}"),
        envelope: EvolutionActivityEnvelopeV1 {
            schema_version: ACTIVITY_SCHEMA_VERSION_V1,
            event_id: format!("event-{sequence}"),
            event_code,
            source_domain: "evidence".into(),
            source_id: format!("source-{sequence}"),
            source_revision: format!("revision-{sequence}"),
            source_sequence: sequence,
            scope_kind: ActivityScopeKind::Workspace,
            canonical_scope_id: "workspace-1".into(),
            occurred_at_ms: committed_at_ms,
            committed_at_ms,
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

fn dashboard_count(connection: &Connection) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM evolution_activity_dashboard_state",
            [],
            |row| row.get(0),
        )
        .expect("dashboard count")
}
