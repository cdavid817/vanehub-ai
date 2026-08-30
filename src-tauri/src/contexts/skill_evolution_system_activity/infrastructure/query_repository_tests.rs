use std::collections::{BTreeMap, BTreeSet};

use rusqlite::Connection;

use super::*;
use crate::contexts::skill_evolution_system_activity::{application::*, domain::*};

#[test]
fn timeline_query_filters_every_safe_indexed_dimension() {
    let (connection, repository, session_id) = fixture();

    assert_sequences(&repository, query(&session_id), &[5, 4, 3, 2, 1]);
    assert_sequences(
        &repository,
        ActivityTimelineQuery {
            committed_from_ms: Some(40),
            committed_to_ms: Some(40),
            ..query(&session_id)
        },
        &[4],
    );
    assert_sequences(
        &repository,
        ActivityTimelineQuery {
            severities: vec![ActivitySeverity::Warning],
            ..query(&session_id)
        },
        &[3, 2],
    );
    assert_sequences(
        &repository,
        ActivityTimelineQuery {
            source_domains: vec![EvolutionSourceDomain::Curator],
            ..query(&session_id)
        },
        &[5],
    );
    assert_sequences(
        &repository,
        ActivityTimelineQuery {
            statuses: vec![ActivityStatus::Blocked],
            ..query(&session_id)
        },
        &[5, 3],
    );
    assert_sequences(
        &repository,
        ActivityTimelineQuery {
            skill_id: Some("SKILL-A".into()),
            ..query(&session_id)
        },
        &[4, 1],
    );
    assert_sequences(
        &repository,
        ActivityTimelineQuery {
            run_id: Some("run-2".into()),
            ..query(&session_id)
        },
        &[2],
    );
    assert_sequences(
        &repository,
        ActivityTimelineQuery {
            curator_states: vec![ActivityCuratorState::Queued],
            ..query(&session_id)
        },
        &[5],
    );
    assert_sequences(
        &repository,
        ActivityTimelineQuery {
            attention_kinds: vec![ActivityAttentionKind::Review],
            ..query(&session_id)
        },
        &[5, 2],
    );
    let _ = connection;
}

#[test]
fn search_uses_alias_codes_and_normalized_identities_but_not_source_or_payload() {
    let (_connection, repository, session_id) = fixture();
    assert_sequences(
        &repository,
        ActivityTimelineQuery {
            search: Some(ActivitySafeSearch {
                event_alias_codes: vec![ActivityEventCode::RunCompleted],
                identity_tokens: Vec::new(),
            }),
            ..query(&session_id)
        },
        &[2],
    );
    assert_sequences(
        &repository,
        ActivityTimelineQuery {
            search: Some(ActivitySafeSearch {
                event_alias_codes: Vec::new(),
                identity_tokens: vec!["skill-a".into()],
            }),
            ..query(&session_id)
        },
        &[4, 1],
    );
    for excluded in ["source-secret", "payload-secret"] {
        assert_sequences(
            &repository,
            ActivityTimelineQuery {
                search: Some(ActivitySafeSearch {
                    event_alias_codes: Vec::new(),
                    identity_tokens: vec![excluded.into()],
                }),
                ..query(&session_id)
            },
            &[],
        );
    }
}

#[test]
fn pagination_is_stable_without_duplicates_and_generation_changes_are_typed() {
    let (connection, repository, session_id) = fixture();
    let mut cursor = None;
    let mut sequences = Vec::new();
    loop {
        let result = repository
            .query_timeline(&ActivityTimelineQuery {
                cursor,
                page_size: 2,
                ..query(&session_id)
            })
            .expect("page");
        let ActivityTimelineQueryResult::Page(page) = result else {
            panic!("current generation must return a page");
        };
        sequences.extend(page.entries.iter().map(|entry| entry.sequence));
        cursor = page.next_cursor;
        if page.complete {
            break;
        }
    }
    assert_eq!(sequences, vec![5, 4, 3, 2, 1]);
    assert_eq!(sequences.iter().copied().collect::<BTreeSet<_>>().len(), 5);

    let first = page(
        repository
            .query_timeline(&ActivityTimelineQuery {
                page_size: 2,
                ..query(&session_id)
            })
            .expect("first page"),
    );
    connection
        .execute(
            "UPDATE evolution_system_activity_sessions SET active_generation_id='generation-next'
             WHERE session_id=?1",
            [&session_id],
        )
        .expect("activate rebuild generation");
    assert_eq!(
        repository
            .query_timeline(&ActivityTimelineQuery {
                cursor: first.next_cursor,
                page_size: 2,
                ..query(&session_id)
            })
            .expect("stale response"),
        ActivityTimelineQueryResult::StaleGeneration {
            requested_generation_id: first.active_generation_id,
            active_generation_id: "generation-next".into(),
        }
    );
}

fn fixture() -> (
    &'static Connection,
    SqliteActivityProjectionRepository<'static>,
    String,
) {
    let connection = Box::new(Connection::open_in_memory().expect("database"));
    apply_schema(&connection).expect("schema");
    let connection: &'static Connection = Box::leak(connection);
    let repository = SqliteActivityProjectionRepository::new(connection);
    let orchestration = vec![
        event(
            1,
            "run-1",
            ActivityEventCode::RunStarted,
            ActivitySeverity::Info,
            ActivityStatus::Running,
            ActivityAttentionKind::None,
            "skill-a",
        ),
        event(
            2,
            "run-2",
            ActivityEventCode::RunCompleted,
            ActivitySeverity::Warning,
            ActivityStatus::Succeeded,
            ActivityAttentionKind::Review,
            "skill-b",
        ),
        event(
            3,
            "run-3",
            ActivityEventCode::RunFailed,
            ActivitySeverity::Warning,
            ActivityStatus::Blocked,
            ActivityAttentionKind::Integrity,
            "skill-c",
        ),
        event(
            4,
            "run-4",
            ActivityEventCode::CuratorApproved,
            ActivitySeverity::Info,
            ActivityStatus::Succeeded,
            ActivityAttentionKind::None,
            "skill-a",
        ),
    ];
    persist_batch(
        &repository,
        EvolutionSourceDomain::Orchestration,
        orchestration,
    );
    persist_batch(
        &repository,
        EvolutionSourceDomain::Curator,
        vec![event(
            1,
            "candidate-1",
            ActivityEventCode::CuratorQueued,
            ActivitySeverity::Error,
            ActivityStatus::Blocked,
            ActivityAttentionKind::Review,
            "skill-d",
        )],
    );
    let mut session_id = String::new();
    for event_id in [
        "event-1",
        "event-2",
        "event-3",
        "event-4",
        "event-1-curator",
    ] {
        session_id = repository
            .deliver_timeline(event_id, 100)
            .expect("delivery")
            .session_id;
    }
    (connection, repository, session_id)
}

fn persist_batch(
    repository: &SqliteActivityProjectionRepository<'_>,
    domain: EvolutionSourceDomain,
    events: Vec<VerifiedProjectionEvent>,
) {
    let last = events.last().expect("event");
    repository
        .commit_projection_batch(&ActivityProjectionBatch {
            checkpoint: ActivityDomainCheckpoint {
                source_domain: domain,
                opaque_cursor: last.source_cursor.clone(),
                last_sequence: last.source_sequence,
                last_source_hash: last.source_integrity_hash.clone(),
                retention_floor: None,
                pending_count: 0,
                oldest_pending_at_ms: None,
                last_success_at_ms: 100,
                expected_revision: 0,
            },
            events,
        })
        .expect("batch");
}

fn event(
    sequence: u64,
    source_id: &str,
    event_code: ActivityEventCode,
    severity: ActivitySeverity,
    status: ActivityStatus,
    attention_kind: ActivityAttentionKind,
    skill_id: &str,
) -> VerifiedProjectionEvent {
    let suffix = if event_code == ActivityEventCode::CuratorQueued {
        "-curator"
    } else {
        ""
    };
    VerifiedProjectionEvent {
        source_cursor: OpaqueDomainCursor::parse(format!("cursor:{source_id}")).expect("cursor"),
        source_sequence: sequence,
        source_integrity_hash: format!("hash:{source_id}"),
        envelope: EvolutionActivityEnvelopeV1 {
            schema_version: 1,
            event_id: format!("event-{sequence}{suffix}"),
            event_code,
            source_domain: if suffix.is_empty() {
                "orchestration"
            } else {
                "curator"
            }
            .into(),
            source_id: format!("source-secret-{source_id}"),
            source_revision: format!("revision-{sequence}"),
            source_sequence: sequence,
            scope_kind: ActivityScopeKind::Workspace,
            canonical_scope_id: "workspace-1".into(),
            occurred_at_ms: i64::try_from(sequence * 10).expect("time"),
            committed_at_ms: i64::try_from(sequence * 10).expect("time"),
            severity,
            status,
            attention_kind,
            safe_actor_kind: ActivityActorKind::System,
            safe_identities: vec![
                SafeIdentity {
                    kind: ActivitySafeIdentityKind::Skill,
                    value: skill_id.into(),
                },
                SafeIdentity {
                    kind: ActivitySafeIdentityKind::Run,
                    value: source_id.into(),
                },
            ],
            metrics: BTreeMap::new(),
            reason_codes: Vec::new(),
            navigation: Some(ActivityNavigation {
                kind: ActivityNavigationKind::Run,
                stable_id: "payload-secret".into(),
                child_id: None,
            }),
            supersedes_event_id: None,
            payload: None,
            projection_policy_version: 1,
            content_hash: String::new(),
        }
        .seal()
        .expect("envelope"),
    }
}

fn query(session_id: &str) -> ActivityTimelineQuery {
    ActivityTimelineQuery {
        session_id: session_id.into(),
        committed_from_ms: None,
        committed_to_ms: None,
        severities: Vec::new(),
        source_domains: Vec::new(),
        statuses: Vec::new(),
        skill_id: None,
        run_id: None,
        curator_states: Vec::new(),
        attention_kinds: Vec::new(),
        search: None,
        cursor: None,
        page_size: 100,
    }
}

fn assert_sequences(
    repository: &SqliteActivityProjectionRepository<'_>,
    query: ActivityTimelineQuery,
    expected: &[u64],
) {
    assert_eq!(
        page(repository.query_timeline(&query).expect("query"))
            .entries
            .iter()
            .map(|entry| entry.sequence)
            .collect::<Vec<_>>(),
        expected
    );
}

fn page(result: ActivityTimelineQueryResult) -> ActivityTimelinePage {
    match result {
        ActivityTimelineQueryResult::Page(page) => page,
        other => panic!("expected page, got {other:?}"),
    }
}
