use std::collections::BTreeMap;

use rusqlite::Connection;

use super::*;
use crate::contexts::skill_evolution_system_activity::{application::*, domain::*};

const SCOPE: &str = "workspace-1";

#[test]
fn json_export_matches_the_filtered_timeline_and_hashes_deterministically() {
    let (connection, session_id) = projected_fixture(4);
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let mut query = timeline_query(&session_id);
    query.severities = vec![ActivitySeverity::Error];

    let first = repository
        .export_activity(
            &request("export-1", query.clone(), ActivityExportFormat::Json),
            &never,
        )
        .expect("export");
    let second = repository
        .export_activity(
            &request("export-2", query.clone(), ActivityExportFormat::Json),
            &never,
        )
        .expect("export again");

    // Filter parity: the export holds exactly the error-severity events the timeline returns.
    let parsed: serde_json::Value = serde_json::from_str(&first.content).expect("json");
    let items = parsed["items"].as_array().expect("items");
    assert_eq!(items.len(), 2);
    for item in items {
        assert_eq!(item["envelope"]["severity"], "error");
    }
    assert_eq!(parsed["manifest"]["complete"], true);
    assert_eq!(
        parsed["manifest"]["redactionVersion"],
        ACTIVITY_EXPORT_REDACTION_VERSION
    );
    // Deterministic: same scope, filters, and items produce the same content hash.
    assert_eq!(first.record.content_hash, second.record.content_hash);
    assert_eq!(first.record.item_count, 2);
}

#[test]
fn markdown_export_localizes_titles_with_a_safe_code_fallback() {
    let (connection, session_id) = projected_fixture(2);
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let mut export_request = request(
        "export-md",
        timeline_query(&session_id),
        ActivityExportFormat::Markdown,
    );
    export_request
        .locale_labels
        .insert("breaker_opened".into(), "断路器已打开".into());

    let document = repository
        .export_activity(&export_request, &never)
        .expect("export");
    assert!(document.content.contains("断路器已打开"));
    // `run_completed` has no label: the safe code stays visible instead of guessed display text.
    assert!(document.content.contains("run_completed"));
    assert!(document.content.contains("- locale: `zh-CN`"));
}

#[test]
fn item_limit_truncates_and_reports_an_incomplete_export() {
    let (connection, session_id) = projected_fixture(5);
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let mut export_request = request(
        "export-limited",
        timeline_query(&session_id),
        ActivityExportFormat::Json,
    );
    export_request.item_limit = 3;

    let document = repository
        .export_activity(&export_request, &never)
        .expect("export");
    assert!(!document.record.complete);
    assert_eq!(document.record.item_count, 3);
    let parsed: serde_json::Value = serde_json::from_str(&document.content).expect("json");
    assert_eq!(parsed["items"].as_array().expect("items").len(), 3);
    assert_eq!(parsed["manifest"]["complete"], false);
}

#[test]
fn size_limit_shrinks_the_document_rather_than_exceeding_it() {
    let (connection, session_id) = projected_fixture(5);
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let mut export_request = request(
        "export-sized",
        timeline_query(&session_id),
        ActivityExportFormat::Markdown,
    );
    export_request.size_limit_bytes = 700;

    let document = repository
        .export_activity(&export_request, &never)
        .expect("export");
    assert!(document.content.len() as u64 <= 700);
    assert!(!document.record.complete);
    assert!(document.record.item_count < 5);
}

#[test]
fn cancellation_stops_the_export_and_records_nothing() {
    let (connection, session_id) = projected_fixture(3);
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let export_request = request(
        "export-cancelled",
        timeline_query(&session_id),
        ActivityExportFormat::Json,
    );

    assert_eq!(
        repository.export_activity(&export_request, &(|| true)),
        Err(ActivityProjectionRepositoryError::Cancelled)
    );
    let recorded: i64 = connection
        .query_row("SELECT COUNT(*) FROM evolution_activity_exports", [], |r| {
            r.get(0)
        })
        .expect("count");
    assert_eq!(recorded, 0);
}

#[test]
fn export_contains_only_safe_envelope_fields() {
    let (connection, session_id) = projected_fixture(2);
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let document = repository
        .export_activity(
            &request(
                "export-safe",
                timeline_query(&session_id),
                ActivityExportFormat::Json,
            ),
            &never,
        )
        .expect("export");
    // The export is built from canonical envelopes only; nothing that looks like source detail,
    // draft content, or diff material can appear because it was never persisted in an envelope.
    for forbidden in [
        "dossier",
        "diff --",
        "draft_body",
        "evidence_excerpt",
        "prompt",
    ] {
        assert!(
            !document.content.contains(forbidden),
            "export leaked {forbidden}"
        );
    }
}

fn never() -> bool {
    false
}

fn timeline_query(session_id: &str) -> ActivityTimelineQuery {
    ActivityTimelineQuery {
        session_id: session_id.to_owned(),
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
        page_size: 50,
    }
}

fn request(
    export_id: &str,
    query: ActivityTimelineQuery,
    format: ActivityExportFormat,
) -> ActivityExportRequest {
    ActivityExportRequest {
        export_id: export_id.to_owned(),
        query,
        format,
        locale: "zh-CN".into(),
        locale_labels: BTreeMap::new(),
        item_limit: 1_000,
        size_limit_bytes: 10 * 1024 * 1024,
        created_at_ms: 2_000,
    }
}

fn projected_fixture(events_count: u64) -> (Connection, String) {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    apply_query_schema(&connection).expect("query schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let events: Vec<VerifiedProjectionEvent> = (1..=events_count).map(event).collect();
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
    let adapter = SqliteActivityTargetDeliveryAdapter::new(&repository, &repository);
    let projector = ActivityTargetProjector::new(&adapter);
    for sequence in 1..=events_count {
        projector
            .project(&format!("event-{sequence}"), 500)
            .expect("projection");
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
