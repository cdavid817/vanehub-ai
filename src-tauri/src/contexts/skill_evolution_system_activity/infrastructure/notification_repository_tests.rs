use std::collections::BTreeMap;

use rusqlite::Connection;

use super::*;
use crate::contexts::skill_evolution_system_activity::{application::*, domain::*};

#[test]
fn immediate_notification_is_deduplicated_and_open_waits_for_visible_timeline() {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);
    persist(
        &repository,
        vec![event(
            1,
            ActivityEventCode::RunFailed,
            ActivitySeverity::Warning,
            ActivityAttentionKind::Security,
        )],
    );
    repository
        .deliver_notification(
            &envelope(&connection, "event-1"),
            "workspace:workspace-1",
            20,
        )
        .expect("notification request");
    assert_eq!(
        count(&connection, "evolution_activity_notification_requests"),
        1
    );
    assert_eq!(
        repository
            .open_notification_after_visible("activity-notification:event-1", "local", 0, 21)
            .expect("pending open"),
        ActivityNotificationOpenOutcome::PendingTimeline
    );

    let delivery = repository
        .deliver_timeline("event-1", 22)
        .expect("timeline");
    repository
        .project_unread(&delivery.session_id, "local", 22)
        .expect("unread");
    let opened = repository
        .open_notification_after_visible("activity-notification:event-1", "local", 1, 23)
        .expect("visible open");
    let ActivityNotificationOpenOutcome::Opened {
        sequence,
        read_state,
        ..
    } = opened
    else {
        panic!("expected opened notification");
    };
    assert_eq!(sequence, 1);
    assert_eq!(read_state.highest_read_sequence, 1);
    assert_eq!(session_unread(&connection), 0);
}

#[test]
fn routine_events_share_a_bounded_digest_while_urgent_event_bypasses_it() {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let mut preferences = default_preferences();
    preferences.digest_cadence = ActivityDigestCadence::Hourly;
    repository
        .update_preferences(&preferences, 1)
        .expect("digest preferences");
    persist(
        &repository,
        vec![
            event(
                1,
                ActivityEventCode::RunCompleted,
                ActivitySeverity::Info,
                ActivityAttentionKind::None,
            ),
            event(
                2,
                ActivityEventCode::RunCompleted,
                ActivitySeverity::Info,
                ActivityAttentionKind::None,
            ),
            event(
                3,
                ActivityEventCode::RunFailed,
                ActivitySeverity::Error,
                ActivityAttentionKind::Integrity,
            ),
        ],
    );
    let adapter = SqliteActivityTargetDeliveryAdapter::new(&repository, &repository);
    let projector = ActivityTargetProjector::new(&adapter);
    for event_id in ["event-1", "event-2", "event-3"] {
        projector.project(event_id, 3_700_000).expect("projection");
    }
    projector
        .project("event-1", 3_700_100)
        .expect("receipt replay");

    assert_eq!(count(&connection, "evolution_activity_digest_buckets"), 1);
    assert_eq!(
        count(&connection, "evolution_activity_notification_requests"),
        1
    );
    let (counts_json, started, ends): (String, i64, i64) = connection
        .query_row(
            "SELECT counts_json,window_started_at_ms,window_ends_at_ms
             FROM evolution_activity_digest_buckets",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("digest");
    assert_eq!(
        serde_json::from_str::<BTreeMap<String, u32>>(&counts_json)
            .expect("counts")
            .get("run_completed"),
        Some(&2)
    );
    assert_eq!((started, ends), (3_600_000, 7_200_000));
    assert_eq!(notification_receipts(&connection), 3);
}

#[test]
fn closed_digest_window_is_claimed_once_with_counts_range_and_severity() {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let mut preferences = default_preferences();
    preferences.digest_cadence = ActivityDigestCadence::Hourly;
    repository
        .update_preferences(&preferences, 1)
        .expect("digest preferences");
    persist(
        &repository,
        vec![
            event(
                1,
                ActivityEventCode::RunCompleted,
                ActivitySeverity::Info,
                ActivityAttentionKind::None,
            ),
            event(
                2,
                ActivityEventCode::CuratorQueued,
                ActivitySeverity::Warning,
                ActivityAttentionKind::None,
            ),
        ],
    );
    let adapter = SqliteActivityTargetDeliveryAdapter::new(&repository, &repository);
    let projector = ActivityTargetProjector::new(&adapter);
    for event_id in ["event-1", "event-2"] {
        projector.project(event_id, 3_700_000).expect("projection");
    }

    // Window [3_600_000, 7_200_000) has not closed yet: nothing is due.
    assert_eq!(
        repository
            .claim_due_digest_notifications(7_199_999)
            .expect("early claim"),
        Vec::new()
    );
    let claimed = repository
        .claim_due_digest_notifications(7_200_000)
        .expect("claim");
    assert_eq!(claimed.len(), 1);
    let digest = &claimed[0];
    assert_eq!(
        (digest.window_started_at_ms, digest.window_ends_at_ms),
        (3_600_000, 7_200_000)
    );
    assert_eq!(digest.cadence, ActivityDigestCadence::Hourly);
    assert_eq!(digest.highest_severity, ActivitySeverity::Warning);
    assert_eq!(digest.counts_by_event_code.get("run_completed"), Some(&1));
    assert_eq!(digest.counts_by_event_code.get("curator_queued"), Some(&1));
    // A second claim — a restart or catch-up replay — must not deliver the same window again.
    assert_eq!(
        repository
            .claim_due_digest_notifications(7_200_001)
            .expect("replayed claim"),
        Vec::new()
    );
}

#[test]
fn dismissal_changes_only_notification_presentation_state() {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);
    persist(
        &repository,
        vec![event(
            1,
            ActivityEventCode::BreakerOpened,
            ActivitySeverity::Error,
            ActivityAttentionKind::Breaker,
        )],
    );
    let adapter = SqliteActivityTargetDeliveryAdapter::new(&repository, &repository);
    ActivityTargetProjector::new(&adapter)
        .project("event-1", 20)
        .expect("projection");
    repository
        .dismiss_notification("activity-notification:event-1", 21)
        .expect("dismiss");
    assert_eq!(session_unread(&connection), 1);
    assert_eq!(count(&connection, "evolution_activity_items"), 1);
    let session_id = connection
        .query_row(
            "SELECT session_id FROM evolution_system_activity_sessions",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("session");
    assert_eq!(
        repository
            .read_state(&session_id, "local")
            .expect("read lookup")
            .expect("read state")
            .highest_read_sequence,
        0
    );
}

fn persist(
    repository: &SqliteActivityProjectionRepository<'_>,
    events: Vec<VerifiedProjectionEvent>,
) {
    let last = events.last().expect("event");
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
                expected_revision: 0,
            },
            events,
        })
        .expect("persist events");
}

fn event(
    sequence: u64,
    event_code: ActivityEventCode,
    severity: ActivitySeverity,
    attention_kind: ActivityAttentionKind,
) -> VerifiedProjectionEvent {
    VerifiedProjectionEvent {
        source_cursor: OpaqueDomainCursor::parse(format!("cursor:{sequence}")).expect("cursor"),
        source_sequence: sequence,
        source_integrity_hash: format!("hash:{sequence}"),
        envelope: EvolutionActivityEnvelopeV1 {
            schema_version: 1,
            event_id: format!("event-{sequence}"),
            event_code,
            source_domain: "orchestration".into(),
            source_id: format!("run-{sequence}"),
            source_revision: format!("revision-{sequence}"),
            source_sequence: sequence,
            scope_kind: ActivityScopeKind::Workspace,
            canonical_scope_id: "workspace-1".into(),
            occurred_at_ms: i64::try_from(sequence).expect("time"),
            committed_at_ms: i64::try_from(sequence).expect("time"),
            severity,
            status: if attention_kind == ActivityAttentionKind::Review {
                ActivityStatus::Blocked
            } else {
                ActivityStatus::Succeeded
            },
            attention_kind,
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

fn envelope(connection: &Connection, event_id: &str) -> EvolutionActivityEnvelopeV1 {
    let json = connection
        .query_row(
            "SELECT envelope_json FROM evolution_activity_envelopes WHERE event_id=?1",
            [event_id],
            |row| row.get::<_, String>(0),
        )
        .expect("envelope json");
    serde_json::from_str(&json).expect("envelope")
}

fn default_preferences() -> EvolutionActivityPreferences {
    EvolutionActivityPreferences {
        scope_kind: ActivityScopeKind::Workspace,
        canonical_scope_id: "workspace-1".into(),
        visible: true,
        minimum_timeline_severity: ActivitySeverity::Info,
        notification_threshold: ActivitySeverity::Warning,
        digest_cadence: ActivityDigestCadence::Off,
        read_retention_days: 180,
        detail_retention_days: 180,
        export_item_limit: 1_000,
        export_size_limit_bytes: 10 * 1024 * 1024,
        revision: 0,
    }
}

fn count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count")
}

fn session_unread(connection: &Connection) -> i64 {
    connection
        .query_row(
            "SELECT unread_count FROM evolution_system_activity_sessions",
            [],
            |row| row.get(0),
        )
        .expect("unread")
}

fn notification_receipts(connection: &Connection) -> i64 {
    connection.query_row(
        "SELECT COUNT(*) FROM evolution_activity_target_receipts WHERE target_kind='notification'",
        [],
        |row| row.get(0),
    ).expect("notification receipts")
}
