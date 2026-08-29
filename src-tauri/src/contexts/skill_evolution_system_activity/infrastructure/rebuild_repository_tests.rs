use std::collections::BTreeMap;

use rusqlite::Connection;

use super::*;
use crate::contexts::skill_evolution_system_activity::{application::*, domain::*};

const SCOPE: &str = "workspace-1";

#[test]
fn corrupted_active_generation_rebuilds_and_activates_a_complete_shadow() {
    let (connection, session_id) = projected_fixture(3);
    let repository = SqliteActivityProjectionRepository::new(&connection);
    // Simulate corruption: the active generation lost an item while envelopes stayed intact.
    connection
        .execute(
            "DELETE FROM evolution_activity_items WHERE event_id='event-2'",
            [],
        )
        .expect("corrupt");

    let rebuild = repository
        .begin_rebuild(ActivityScopeKind::Workspace, SCOPE, 100, 1_000)
        .expect("begin");
    assert_eq!(rebuild.status, ActivityRebuildStatus::Running);
    drive_to_ready(&repository, &rebuild.rebuild_id);
    assert_eq!(
        repository
            .activate_rebuild(&rebuild.rebuild_id, 1_010)
            .expect("activate"),
        ActivityRebuildStep::Active
    );

    let (active_generation, last_sequence): (String, i64) = connection
        .query_row(
            "SELECT active_generation_id,last_sequence FROM evolution_system_activity_sessions
             WHERE session_id=?1",
            [&session_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("session");
    assert_eq!(active_generation, rebuild.shadow_generation_id);
    assert_eq!(last_sequence, 3);
}

#[test]
fn failed_validation_keeps_the_prior_generation_active() {
    let (connection, session_id) = projected_fixture(2);
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let prior_generation = active_generation(&connection, &session_id);
    let rebuild = repository
        .begin_rebuild(ActivityScopeKind::Workspace, SCOPE, 100, 1_000)
        .expect("begin");
    while matches!(
        repository
            .advance_rebuild(&rebuild.rebuild_id, 10, 1_001)
            .expect("advance"),
        ActivityRebuildStep::Running { .. }
    ) {}
    // Tamper with the shadow so validation's count check must fail.
    connection
        .execute(
            "DELETE FROM evolution_activity_items WHERE generation_id=?1 AND sequence=2",
            [&rebuild.shadow_generation_id],
        )
        .expect("tamper");
    assert_eq!(
        repository.validate_rebuild(&rebuild.rebuild_id, 1_002),
        Err(ActivityProjectionRepositoryError::Conflict)
    );
    assert_eq!(
        active_generation(&connection, &session_id),
        prior_generation
    );
    assert_eq!(
        repository
            .rebuild(&rebuild.rebuild_id)
            .expect("lookup")
            .expect("rebuild")
            .status,
        ActivityRebuildStatus::Failed
    );
}

#[test]
fn events_committed_during_rebuild_force_catch_up_before_activation() {
    let (connection, session_id) = projected_fixture(2);
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let rebuild = repository
        .begin_rebuild(ActivityScopeKind::Workspace, SCOPE, 100, 1_000)
        .expect("begin");
    drive_to_ready(&repository, &rebuild.rebuild_id);

    // A new source event commits after validation but before activation.
    persist(&repository, vec![event(3)]);
    project(&repository, "event-3");

    assert_eq!(
        repository
            .activate_rebuild(&rebuild.rebuild_id, 1_010)
            .expect("gated activation"),
        ActivityRebuildStep::NeedsCatchUp
    );
    drive_to_ready(&repository, &rebuild.rebuild_id);
    assert_eq!(
        repository
            .activate_rebuild(&rebuild.rebuild_id, 1_020)
            .expect("activate"),
        ActivityRebuildStep::Active
    );
    let shadow_items: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM evolution_activity_items WHERE generation_id=?1",
            [&rebuild.shadow_generation_id],
            |row| row.get(0),
        )
        .expect("shadow count");
    assert_eq!(shadow_items, 3);
    let _ = session_id;
}

#[test]
fn cancellation_removes_the_shadow_and_leaves_the_active_generation_untouched() {
    let (connection, session_id) = projected_fixture(2);
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let prior_generation = active_generation(&connection, &session_id);
    let rebuild = repository
        .begin_rebuild(ActivityScopeKind::Workspace, SCOPE, 100, 1_000)
        .expect("begin");
    repository
        .advance_rebuild(&rebuild.rebuild_id, 1, 1_001)
        .expect("advance");
    repository
        .cancel_rebuild(&rebuild.rebuild_id, 1_002)
        .expect("cancel");

    let shadow_items: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM evolution_activity_items WHERE generation_id=?1",
            [&rebuild.shadow_generation_id],
            |row| row.get(0),
        )
        .expect("shadow count");
    assert_eq!(shadow_items, 0);
    assert_eq!(
        active_generation(&connection, &session_id),
        prior_generation
    );
    assert_eq!(
        repository.advance_rebuild(&rebuild.rebuild_id, 1, 1_003),
        Err(ActivityProjectionRepositoryError::Conflict)
    );
}

#[test]
fn activation_is_refused_before_validation_so_a_crash_leaves_the_prior_generation() {
    let (connection, session_id) = projected_fixture(2);
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let prior_generation = active_generation(&connection, &session_id);
    let rebuild = repository
        .begin_rebuild(ActivityScopeKind::Workspace, SCOPE, 100, 1_000)
        .expect("begin");
    // A crash between advancing and validating leaves status short of ready: activation must
    // refuse rather than switch to an unvalidated shadow.
    assert_eq!(
        repository.activate_rebuild(&rebuild.rebuild_id, 1_001),
        Err(ActivityProjectionRepositoryError::Conflict)
    );
    assert_eq!(
        active_generation(&connection, &session_id),
        prior_generation
    );
}

#[test]
fn rebuild_preserves_read_state_and_never_replays_notifications() {
    let (connection, session_id) = projected_fixture(3);
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let read = repository
        .project_unread(&session_id, LOCAL_ACTIVITY_USER_ID, 900)
        .expect("unread");
    repository
        .advance_read_cursor(&session_id, LOCAL_ACTIVITY_USER_ID, 2, read.revision, 901)
        .expect("read through 2");
    let receipts_before = notification_receipts(&connection);

    let rebuild = repository
        .begin_rebuild(ActivityScopeKind::Workspace, SCOPE, 100, 1_000)
        .expect("begin");
    drive_to_ready(&repository, &rebuild.rebuild_id);
    repository
        .activate_rebuild(&rebuild.rebuild_id, 1_010)
        .expect("activate");

    let read_after = repository
        .read_state(&session_id, LOCAL_ACTIVITY_USER_ID)
        .expect("read lookup")
        .expect("read state");
    assert_eq!(read_after.highest_read_sequence, 2);
    assert_eq!(notification_receipts(&connection), receipts_before);
    let unread: i64 = connection
        .query_row(
            "SELECT unread_count FROM evolution_system_activity_sessions WHERE session_id=?1",
            [&session_id],
            |row| row.get(0),
        )
        .expect("unread");
    assert_eq!(unread, 1);
}

#[test]
fn prior_generation_stays_readable_through_the_recovery_window() {
    let (connection, session_id) = projected_fixture(2);
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let prior_generation = active_generation(&connection, &session_id);
    let rebuild = repository
        .begin_rebuild(ActivityScopeKind::Workspace, SCOPE, 100, 1_000)
        .expect("begin");
    drive_to_ready(&repository, &rebuild.rebuild_id);
    repository
        .activate_rebuild(&rebuild.rebuild_id, 1_010)
        .expect("activate");

    let prior_items: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM evolution_activity_items WHERE generation_id=?1",
            [&prior_generation],
            |row| row.get(0),
        )
        .expect("prior items");
    assert_eq!(prior_items, 2);
}

fn projected_fixture(events_count: u64) -> (Connection, String) {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);
    persist(&repository, (1..=events_count).map(event).collect());
    for sequence in 1..=events_count {
        project(&repository, &format!("event-{sequence}"));
    }
    let session_id = connection
        .query_row(
            "SELECT session_id FROM evolution_system_activity_sessions",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("session");
    (connection, session_id)
}

fn project(repository: &SqliteActivityProjectionRepository<'_>, event_id: &str) {
    let adapter = SqliteActivityTargetDeliveryAdapter::new(repository, repository);
    ActivityTargetProjector::new(&adapter)
        .project(event_id, 500)
        .expect("projection");
}

fn drive_to_ready(repository: &SqliteActivityProjectionRepository<'_>, rebuild_id: &str) {
    loop {
        match repository
            .advance_rebuild(rebuild_id, 10, 1_001)
            .expect("advance")
        {
            ActivityRebuildStep::Running { .. } => {}
            ActivityRebuildStep::Validating => break,
            step => panic!("unexpected step {step:?}"),
        }
    }
    assert_eq!(
        repository
            .validate_rebuild(rebuild_id, 1_002)
            .expect("validate"),
        ActivityRebuildStep::Ready
    );
}

fn active_generation(connection: &Connection, session_id: &str) -> String {
    connection
        .query_row(
            "SELECT active_generation_id FROM evolution_system_activity_sessions
             WHERE session_id=?1",
            [session_id],
            |row| row.get(0),
        )
        .expect("generation")
}

fn notification_receipts(connection: &Connection) -> i64 {
    connection.query_row(
        "SELECT COUNT(*) FROM evolution_activity_target_receipts WHERE target_kind='notification'",
        [],
        |row| row.get(0),
    ).expect("notification receipts")
}

fn persist(
    repository: &SqliteActivityProjectionRepository<'_>,
    events: Vec<VerifiedProjectionEvent>,
) {
    let last = events.last().expect("event");
    let expected_revision = repository
        .cursor(EvolutionSourceDomain::Orchestration)
        .expect("cursor lookup")
        .map(|cursor| cursor.revision)
        .unwrap_or(0);
    repository
        .commit_projection_batch(&ActivityProjectionBatch {
            checkpoint: ActivityDomainCheckpoint {
                source_domain: EvolutionSourceDomain::Orchestration,
                opaque_cursor: last.source_cursor.clone(),
                last_sequence: last.source_sequence,
                last_source_hash: last.source_integrity_hash.clone(),
                retention_floor: None,
                pending_count: 0,
                oldest_pending_at_ms: None,
                last_success_at_ms: 10,
                expected_revision,
            },
            events,
        })
        .expect("persist events");
}

fn event(sequence: u64) -> VerifiedProjectionEvent {
    VerifiedProjectionEvent {
        source_cursor: OpaqueDomainCursor::parse(format!("cursor:{sequence}")).expect("cursor"),
        source_sequence: sequence,
        source_integrity_hash: format!("hash:{sequence}"),
        envelope: EvolutionActivityEnvelopeV1 {
            schema_version: 1,
            event_id: format!("event-{sequence}"),
            event_code: if sequence.is_multiple_of(2) {
                ActivityEventCode::RunCompleted
            } else {
                ActivityEventCode::BreakerOpened
            },
            source_domain: "orchestration".into(),
            source_id: format!("run-{sequence}"),
            source_revision: format!("revision-{sequence}"),
            source_sequence: sequence,
            scope_kind: ActivityScopeKind::Workspace,
            canonical_scope_id: SCOPE.into(),
            occurred_at_ms: i64::try_from(sequence).expect("time"),
            committed_at_ms: i64::try_from(sequence).expect("time"),
            severity: if sequence.is_multiple_of(2) {
                ActivitySeverity::Info
            } else {
                ActivitySeverity::Error
            },
            status: ActivityStatus::Succeeded,
            attention_kind: if sequence.is_multiple_of(2) {
                ActivityAttentionKind::None
            } else {
                ActivityAttentionKind::Breaker
            },
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
