use std::collections::BTreeMap;

use rusqlite::Connection;

use super::*;
use crate::contexts::skill_evolution_system_activity::{application::*, domain::*};

#[test]
fn unread_projection_is_exact_and_uses_highest_retained_attention() {
    let (connection, session_id) = fixture();
    let repository = SqliteActivityProjectionRepository::new(&connection);

    let state = repository
        .project_unread(&session_id, LOCAL_ACTIVITY_USER_ID, 30)
        .expect("unread projection");

    assert_eq!(state.highest_read_sequence, 0);
    assert_eq!(session_summary(&connection), (2, "security".into()));
}

#[test]
fn read_cursor_is_monotonic_and_mark_unread_is_bounded() {
    let (connection, session_id) = fixture();
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let initial = repository
        .project_unread(&session_id, LOCAL_ACTIVITY_USER_ID, 30)
        .expect("unread projection");
    let through_first = repository
        .advance_read_cursor(&session_id, LOCAL_ACTIVITY_USER_ID, 1, initial.revision, 40)
        .expect("read first");
    assert_eq!(session_summary(&connection), (1, "review".into()));

    assert_eq!(
        repository.advance_read_cursor(
            &session_id,
            LOCAL_ACTIVITY_USER_ID,
            2,
            initial.revision,
            41,
        ),
        Err(ActivityProjectionRepositoryError::Conflict)
    );
    let unread = repository
        .mark_unread(
            &session_id,
            LOCAL_ACTIVITY_USER_ID,
            1,
            through_first.revision,
            50,
        )
        .expect("mark unread");
    assert_eq!(unread.mark_unread_sequence, Some(1));
    assert_eq!(session_summary(&connection), (2, "security".into()));

    assert_eq!(
        repository.mark_unread(&session_id, LOCAL_ACTIVITY_USER_ID, 3, unread.revision, 51,),
        Err(ActivityProjectionRepositoryError::InvalidInput)
    );
    let all_read = repository
        .advance_read_cursor(&session_id, LOCAL_ACTIVITY_USER_ID, 2, unread.revision, 60)
        .expect("read all");
    assert_eq!(all_read.highest_read_sequence, 2);
    assert_eq!(all_read.mark_unread_sequence, None);
    assert_eq!(session_summary(&connection), (0, "none".into()));

    let monotonic = repository
        .advance_read_cursor(
            &session_id,
            LOCAL_ACTIVITY_USER_ID,
            1,
            all_read.revision,
            70,
        )
        .expect("older cursor is harmless");
    assert_eq!(monotonic.highest_read_sequence, 2);
}

fn fixture() -> (Connection, String) {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let setup = SqliteActivityProjectionRepository::new(&connection);
    persist(
        &setup,
        event(1, "event-1", ActivityAttentionKind::Security),
        0,
    );
    let first = setup
        .deliver_timeline("event-1", 20)
        .expect("first timeline item");
    persist(
        &setup,
        event(2, "event-2", ActivityAttentionKind::Review),
        1,
    );
    setup
        .deliver_timeline("event-2", 21)
        .expect("second timeline item");
    (connection, first.session_id)
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
    event_id: &str,
    attention_kind: ActivityAttentionKind,
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
            occurred_at_ms: sequence as i64,
            committed_at_ms: sequence as i64,
            severity: ActivitySeverity::Warning,
            status: ActivityStatus::Succeeded,
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

fn session_summary(connection: &Connection) -> (i64, String) {
    connection
        .query_row(
            "SELECT unread_count,attention_kind FROM evolution_system_activity_sessions",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("session summary")
}
