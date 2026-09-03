use super::cursor::{filter_fingerprint, RecordCursor};
use super::{repair_missing_evidence_schema, SqliteEvidenceRepository};
use crate::contexts::execution_observability::application::evidence::models::{
    EvidenceQueryScope, ExecutionRecordDetailQuery, ExecutionRecordFilters, ExecutionRecordKind,
    ExecutionRecordQuery, WorkspaceEvidenceSummaryQuery, MAX_EVIDENCE_PAGE_SIZE,
};
use crate::contexts::execution_observability::application::evidence::ports::{
    EvidenceAppendOutcome, EvidenceRepositoryPort,
};
use crate::contexts::execution_observability::application::EvidenceApplicationError;
use crate::contexts::execution_observability::domain::evidence::builders::{
    label, CorrelationBuilder, EvidenceEventBuilder,
};
use crate::contexts::execution_observability::domain::evidence::payload::EvidenceOutcome;
use crate::contexts::execution_observability::domain::evidence::safety::RedactedCommandDisplay;
use crate::contexts::execution_observability::domain::{
    reason_codes, CommandRuntimeKind, EvidenceCoverageState, EvidenceSeatId, EvidenceSessionId,
    EvidenceSourceContext, ExecutionEvidenceEvent, ExecutionFidelity, ExecutionStatus,
    OutputAvailability, SafeEvidencePayload, SafeReasonCode, VerificationOutcome,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

const SESSION: &str = "session-1";
const RUN: &str = "6f1b2c3d-4e5f-4a6b-8c9d-0e1f2a3b4c5d";
const TRACE: &str = "0af7651916cd43dd8448eb211c80319c";

fn repository(name: &str) -> (TempDirectory, NativeDatabase, SqliteEvidenceRepository) {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let repository = SqliteEvidenceRepository::new(database.clone());
    (directory, database, repository)
}

fn session() -> EvidenceSessionId {
    EvidenceSessionId::parse(SESSION).expect("session")
}

fn command_started(
    source_event_id: &str,
    command_id: &str,
    occurred_at: &str,
) -> ExecutionEvidenceEvent {
    EvidenceEventBuilder::new(
        source_event_id,
        CorrelationBuilder::for_session(SESSION)
            .with_run(RUN, TRACE)
            .with_command(command_id)
            .build(),
        SafeEvidencePayload::CommandStarted {
            runtime_kind: CommandRuntimeKind::LocalShell,
            redacted_display: Some(RedactedCommandDisplay::parse("npm test").expect("display")),
            cwd_display: None,
        },
    )
    .with_status(ExecutionStatus::Running)
    .with_occurred_at(occurred_at)
    .build()
}

fn command_completed(
    source_event_id: &str,
    command_id: &str,
    occurred_at: &str,
    exit_code: i32,
) -> ExecutionEvidenceEvent {
    EvidenceEventBuilder::new(
        source_event_id,
        CorrelationBuilder::for_session(SESSION)
            .with_run(RUN, TRACE)
            .with_command(command_id)
            .build(),
        SafeEvidencePayload::CommandCompleted {
            outcome: EvidenceOutcome::Failed,
            duration_ms: Some(12_400),
            exit_code: Some(exit_code),
            signal: None,
            output_availability: OutputAvailability::Merged,
            output_truncated: true,
        },
    )
    .with_status(ExecutionStatus::Failed)
    .with_occurred_at(occurred_at)
    .build()
}

fn append(
    repository: &SqliteEvidenceRepository,
    event: &ExecutionEvidenceEvent,
) -> EvidenceAppendOutcome {
    repository
        .append(
            event,
            &event.canonical_fingerprint(),
            "2026-01-01T00:00:00Z",
        )
        .expect("append")
}

fn query(limit: usize) -> ExecutionRecordQuery {
    ExecutionRecordQuery {
        scope: EvidenceQueryScope {
            session_id: Some(session()),
            ..EvidenceQueryScope::default()
        },
        filters: ExecutionRecordFilters::default(),
        cursor: None,
        limit,
    }
}

#[test]
fn the_migration_creates_the_journal_projection_coverage_and_indexes() {
    let (_directory, database, _repository) = repository("evidence-schema");
    let connection = database.connection().expect("connection");

    for table in [
        "execution_evidence_events",
        "execution_evidence_records",
        "execution_evidence_coverage",
    ] {
        let present: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                rusqlite::params![table],
                |row| row.get(0),
            )
            .expect("table lookup");
        assert_eq!(present, 1, "{table} must exist after migration");
    }

    for index in [
        "idx_execution_evidence_session_sequence",
        "idx_execution_evidence_run_sequence",
        "idx_execution_evidence_trace_span",
        "idx_execution_evidence_operation",
        "idx_execution_evidence_retention",
        "idx_evidence_records_session_page",
        "idx_evidence_records_seat_page",
        "idx_evidence_records_run_page",
        "idx_evidence_records_retention",
    ] {
        let present: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = ?1",
                rusqlite::params![index],
                |row| row.get(0),
            )
            .expect("index lookup");
        assert_eq!(present, 1, "{index} must exist after migration");
    }
}

// The page query is the one that runs on every keystroke in Terminal History; a plan that scans
// would only show up under load, long after the change shipped.
#[test]
fn the_newest_first_page_query_uses_its_index() {
    let (_directory, database, _repository) = repository("evidence-plan");
    let connection = database.connection().expect("connection");
    let plan: String = connection
        .query_row(
            "EXPLAIN QUERY PLAN SELECT record_id FROM execution_evidence_records \
             WHERE session_id = 'session-1' ORDER BY occurred_at DESC, record_id DESC LIMIT 10",
            [],
            |row| row.get(3),
        )
        .expect("query plan");
    assert!(
        plan.contains("idx_evidence_records_session_page"),
        "expected the session page index, got: {plan}"
    );
}

#[test]
fn the_schema_survives_a_reopen_and_the_repair_is_idempotent() {
    let directory = TempDirectory::new("evidence-reopen");
    {
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let repository = SqliteEvidenceRepository::new(database);
        append(
            &repository,
            &command_started("source-1", "command-1", "2026-01-01T00:00:00Z"),
        );
    }
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("reopen");
    let connection = database.connection().expect("connection");
    // Running the repair on a database that already has the schema must change nothing.
    repair_missing_evidence_schema(&connection).expect("repair");
    let events: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM execution_evidence_events",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(
        events, 1,
        "reopening must not lose or duplicate the journal"
    );
}

#[test]
fn an_identical_retry_neither_inserts_nor_projects_again() {
    let (_directory, database, repository) = repository("evidence-duplicate");
    let event = command_completed("source-1", "command-1", "2026-01-01T00:00:00Z", 1);

    let first = append(&repository, &event);
    let second = append(&repository, &event);

    assert!(matches!(first, EvidenceAppendOutcome::Appended { .. }));
    let EvidenceAppendOutcome::IdenticalDuplicate { sequence } = second else {
        // Named rather than formatted whole. The outcome carries only a sequence number, so
        // printing it is harmless — but `{:?}` on a value that reached this code from a producer is
        // the shape that puts payloads in a panic message once somebody adds a field, and the
        // variant name is the entire diagnostic here anyway.
        panic!("a retry must be reported as an idempotent duplicate, got a different variant");
    };
    let EvidenceAppendOutcome::Appended {
        sequence: first_sequence,
    } = first
    else {
        unreachable!()
    };
    assert_eq!(
        sequence, first_sequence,
        "the retry reports the original row"
    );

    let connection = database.connection().expect("connection");
    let events: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM execution_evidence_events",
            [],
            |row| row.get(0),
        )
        .expect("count");
    let records: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM execution_evidence_records",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(events, 1);
    assert_eq!(records, 1);
}

/// A failed insert must leave nothing behind.
///
/// The journal row, the projection, and the coverage metadata are written in one transaction
/// precisely so a half-applied append cannot exist: a journal row whose projection never landed
/// would be invisible to every query while still counting toward completeness.
#[test]
fn a_failed_append_leaves_no_partial_state() {
    let (_directory, database, repository) = repository("evidence-rollback");
    append(
        &repository,
        &command_started("source-1", "command-1", "2026-01-01T00:00:00Z"),
    );

    // A second event reusing the generated event id violates the journal's UNIQUE constraint, so
    // the insert fails after the transaction has opened.
    let colliding = EvidenceEventBuilder::new(
        "source-2",
        CorrelationBuilder::for_session(SESSION)
            .with_run(RUN, TRACE)
            .with_command("command-2")
            .build(),
        SafeEvidencePayload::CommandStarted {
            runtime_kind: CommandRuntimeKind::Process,
            redacted_display: None,
            cwd_display: None,
        },
    )
    .with_event_id("event-source-1")
    .with_status(ExecutionStatus::Running)
    .with_occurred_at("2026-01-01T00:00:05Z")
    .build();
    let error = repository
        .append(
            &colliding,
            &colliding.canonical_fingerprint(),
            "2026-01-01T00:00:05Z",
        )
        .expect_err("the colliding insert fails");
    assert!(matches!(error, EvidenceApplicationError::Storage(_)));

    let connection = database.connection().expect("connection");
    let events: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM execution_evidence_events",
            [],
            |row| row.get(0),
        )
        .expect("count");
    let records: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM execution_evidence_records",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(events, 1, "the failed event is not in the journal");
    assert_eq!(records, 1, "and it produced no projection row");
}

#[test]
fn a_conflicting_source_id_preserves_the_original_and_marks_coverage_partial() {
    let (_directory, database, repository) = repository("evidence-conflict");
    let original = command_completed("source-1", "command-1", "2026-01-01T00:00:00Z", 1);
    append(&repository, &original);

    // Same source id, different asserted content.
    let conflicting = command_completed("source-1", "command-1", "2026-01-01T00:00:00Z", 137);
    let outcome = append(&repository, &conflicting);

    assert_eq!(outcome, EvidenceAppendOutcome::Conflict);
    let connection = database.connection().expect("connection");
    let stored_exit: Option<i64> = connection
        .query_row(
            "SELECT exit_code FROM execution_evidence_records WHERE record_id = 'command:command-1'",
            [],
            |row| row.get(0),
        )
        .expect("record");
    assert_eq!(stored_exit, Some(1), "the original assertion wins");

    let page = repository.list_records(&query(10)).expect("page");
    assert_eq!(page.coverage.state(), EvidenceCoverageState::Partial);
    assert!(page
        .coverage
        .reason_codes()
        .iter()
        .map(SafeReasonCode::as_str)
        .any(|code| code == reason_codes::CONFLICTING_SOURCE_EVENT));

    // Nothing about either payload is stored alongside the conflict counter.
    let coverage_row: String = connection
        .query_row(
            "SELECT COALESCE(GROUP_CONCAT(name), '') FROM pragma_table_info('execution_evidence_coverage')",
            [],
            |row| row.get(0),
        )
        .expect("columns");
    assert!(!coverage_row.contains("payload"));
}

#[test]
fn a_late_start_cannot_overwrite_the_completion_that_already_landed() {
    let (_directory, _database, repository) = repository("evidence-monotonic");
    append(
        &repository,
        &command_completed("source-complete", "command-1", "2026-01-01T00:00:05Z", 1),
    );
    append(
        &repository,
        &command_started("source-start", "command-1", "2026-01-01T00:00:00Z"),
    );

    let page = repository.list_records(&query(10)).expect("page");
    let record = &page.items[0];
    assert_eq!(
        record.status,
        ExecutionStatus::Failed,
        "terminal state survives"
    );
    // The start still contributed what only it observed.
    assert_eq!(record.started_at.as_deref(), Some("2026-01-01T00:00:00Z"));
    assert_eq!(record.ended_at.as_deref(), Some("2026-01-01T00:00:05Z"));
}

// A completion with no start must not borrow its own timestamp as a start time.
#[test]
fn a_completion_without_a_start_leaves_the_start_absent() {
    let (_directory, _database, repository) = repository("evidence-startless");
    append(
        &repository,
        &command_completed("source-1", "command-1", "2026-01-01T00:00:05Z", 1),
    );

    let page = repository.list_records(&query(10)).expect("page");
    let record = &page.items[0];
    assert!(record.started_at.is_none(), "no start was observed");
    assert_eq!(record.ended_at.as_deref(), Some("2026-01-01T00:00:05Z"));
}

/// Task 0.7's evidence pagination fixture.
///
/// Loads a newest-first first page, appends newer evidence, then continues with the original
/// cursor. The boundary the cursor names must still hold: no record that existed at that boundary
/// may be repeated or skipped, and the rows that arrived afterwards belong to a later refresh, not
/// to this continuation.
#[test]
fn a_cursor_keeps_its_boundary_when_newer_evidence_arrives_between_pages() {
    let (_directory, _database, repository) = repository("evidence-keyset");
    for index in 0..6 {
        append(
            &repository,
            &command_started(
                &format!("source-{index}"),
                &format!("command-{index}"),
                &format!("2026-01-01T00:00:0{index}Z"),
            ),
        );
    }

    let first = repository.list_records(&query(3)).expect("first page");
    let first_ids = first
        .items
        .iter()
        .map(|record| record.record_id.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        first_ids,
        vec![
            "command:command-5",
            "command:command-4",
            "command:command-3"
        ],
        "the first page is newest first"
    );
    let cursor = first.next_cursor.clone().expect("a further page exists");

    for index in 6..9 {
        append(
            &repository,
            &command_started(
                &format!("source-{index}"),
                &format!("command-{index}"),
                &format!("2026-01-01T00:00:0{index}Z"),
            ),
        );
    }

    let second = repository
        .list_records(&ExecutionRecordQuery {
            cursor: Some(cursor),
            ..query(3)
        })
        .expect("second page");
    let second_ids = second
        .items
        .iter()
        .map(|record| record.record_id.clone())
        .collect::<Vec<_>>();

    assert_eq!(
        second_ids,
        vec![
            "command:command-2",
            "command:command-1",
            "command:command-0"
        ],
        "the continuation resumes exactly after the original boundary"
    );
    let overlap = first_ids
        .iter()
        .filter(|id| second_ids.contains(id))
        .count();
    assert_eq!(
        overlap, 0,
        "no record that existed at the boundary is repeated"
    );
    // The three newer records are not silently absorbed into the continuation; they belong to a
    // refresh of the first page.
    assert!(!second_ids.iter().any(|id| id.contains("command-6")));
}

#[test]
fn a_cursor_issued_for_one_filter_set_is_refused_with_another() {
    let (_directory, _database, repository) = repository("evidence-cursor-mismatch");
    for index in 0..4 {
        append(
            &repository,
            &command_started(
                &format!("source-{index}"),
                &format!("command-{index}"),
                &format!("2026-01-01T00:00:0{index}Z"),
            ),
        );
    }
    let first = repository.list_records(&query(2)).expect("first page");
    let cursor = first.next_cursor.expect("cursor");

    let error = repository
        .list_records(&ExecutionRecordQuery {
            cursor: Some(cursor.clone()),
            filters: ExecutionRecordFilters {
                statuses: vec![ExecutionStatus::Failed],
                ..ExecutionRecordFilters::default()
            },
            ..query(2)
        })
        .expect_err("a cursor cannot cross a filter change");
    assert_eq!(error, EvidenceApplicationError::CursorFilterMismatch);

    // A different scope is equally disqualifying, and neither case falls back to an offset.
    let scoped = repository.list_records(&ExecutionRecordQuery {
        cursor: Some(cursor),
        scope: EvidenceQueryScope {
            session_id: Some(session()),
            seat_id: Some(
                crate::contexts::execution_observability::domain::EvidenceSeatId::parse("seat-a")
                    .expect("seat"),
            ),
            ..EvidenceQueryScope::default()
        },
        ..query(2)
    });
    assert_eq!(
        scoped.expect_err("scope change"),
        EvidenceApplicationError::CursorFilterMismatch
    );
}

#[test]
fn a_malformed_cursor_is_rejected_rather_than_read_as_an_offset() {
    let (_directory, _database, repository) = repository("evidence-cursor-garbage");
    let error = repository
        .list_records(&ExecutionRecordQuery {
            cursor: Some("12".to_string()),
            ..query(2)
        })
        .expect_err("garbage cursor");
    assert_eq!(error, EvidenceApplicationError::InvalidCursor);
}

#[test]
fn the_filter_fingerprint_is_stable_and_scope_sensitive() {
    let base = query(10);
    assert_eq!(filter_fingerprint(&base), filter_fingerprint(&query(10)));
    // The page size is not part of identity: asking for more rows does not change which rows match.
    assert_eq!(filter_fingerprint(&base), filter_fingerprint(&query(50)));

    let filtered = ExecutionRecordQuery {
        filters: ExecutionRecordFilters {
            kinds: vec![ExecutionRecordKind::Command],
            ..ExecutionRecordFilters::default()
        },
        ..query(10)
    };
    assert_ne!(filter_fingerprint(&base), filter_fingerprint(&filtered));

    // Filter order must not change identity, or an equivalent query would reject its own cursor.
    let reordered = ExecutionRecordQuery {
        filters: ExecutionRecordFilters {
            kinds: vec![ExecutionRecordKind::Tool, ExecutionRecordKind::Command],
            ..ExecutionRecordFilters::default()
        },
        ..query(10)
    };
    let same_set = ExecutionRecordQuery {
        filters: ExecutionRecordFilters {
            kinds: vec![ExecutionRecordKind::Command, ExecutionRecordKind::Tool],
            ..ExecutionRecordFilters::default()
        },
        ..query(10)
    };
    assert_eq!(
        filter_fingerprint(&reordered),
        filter_fingerprint(&same_set)
    );
}

#[test]
fn a_cursor_round_trips_and_refuses_a_foreign_fingerprint() {
    let cursor = RecordCursor {
        occurred_at: "2026-01-01T00:00:00Z".to_string(),
        record_id: "command:command-1".to_string(),
        filter_fingerprint: "abc123".to_string(),
    };
    let encoded = cursor.encode();
    assert!(!encoded.contains("command-1"), "the cursor is opaque");
    assert_eq!(
        RecordCursor::decode(&encoded, "abc123").expect("decode"),
        cursor
    );
    assert_eq!(
        RecordCursor::decode(&encoded, "different").expect_err("mismatch"),
        EvidenceApplicationError::CursorFilterMismatch
    );
}

#[test]
fn a_page_never_returns_more_than_the_requested_bound() {
    let (_directory, _database, repository) = repository("evidence-bound");
    for index in 0..5 {
        append(
            &repository,
            &command_started(
                &format!("source-{index}"),
                &format!("command-{index}"),
                &format!("2026-01-01T00:00:0{index}Z"),
            ),
        );
    }
    let page = repository.list_records(&query(2)).expect("page");
    assert_eq!(page.items.len(), 2);
    assert!(page.coverage.truncated());
    assert!(page.next_cursor.is_some());

    let full = repository
        .list_records(&query(MAX_EVIDENCE_PAGE_SIZE))
        .expect("page");
    assert_eq!(full.items.len(), 5);
    assert!(!full.coverage.truncated());
    assert!(full.next_cursor.is_none());
}

// Before Task Group 4 wires a producer, an empty store must not read as "nothing ran".
#[test]
fn an_empty_session_reports_capture_not_initialized_rather_than_complete() {
    let (_directory, _database, repository) = repository("evidence-empty");
    let page = repository.list_records(&query(10)).expect("page");
    assert!(page.items.is_empty());
    assert_eq!(page.coverage.state(), EvidenceCoverageState::Partial);
    assert!(page
        .coverage
        .reason_codes()
        .iter()
        .map(SafeReasonCode::as_str)
        .any(|code| code == reason_codes::CAPTURE_NOT_INITIALIZED));
}

#[test]
fn a_session_with_evidence_reports_complete_coverage_and_its_boundaries() {
    let (_directory, _database, repository) = repository("evidence-complete");
    append(
        &repository,
        &command_started("source-1", "command-1", "2026-01-01T00:00:00Z"),
    );
    append(
        &repository,
        &command_started("source-2", "command-2", "2026-01-01T00:00:05Z"),
    );

    let page = repository.list_records(&query(10)).expect("page");
    assert_eq!(page.coverage.state(), EvidenceCoverageState::Complete);
    assert_eq!(
        page.coverage.oldest_available_at(),
        Some("2026-01-01T00:00:00Z")
    );
    assert_eq!(
        page.coverage.newest_available_at(),
        Some("2026-01-01T00:00:05Z")
    );
}

#[test]
fn a_record_detail_carries_correlation_counts_and_a_failure_reason() {
    let (_directory, _database, repository) = repository("evidence-detail");
    append(
        &repository,
        &command_completed("source-1", "command-1", "2026-01-01T00:00:05Z", 1),
    );
    append(
        &repository,
        &EvidenceEventBuilder::new(
            "source-tool",
            CorrelationBuilder::for_session(SESSION)
                .with_run(RUN, TRACE)
                .with_tool_call("tool-1")
                .build(),
            SafeEvidencePayload::ToolCompleted {
                tool_name: label("read_file"),
                outcome: EvidenceOutcome::Succeeded,
                duration_ms: Some(31),
            },
        )
        .with_status(ExecutionStatus::Succeeded)
        .with_occurred_at("2026-01-01T00:00:06Z")
        .build(),
    );

    let detail = repository
        .record_detail(&ExecutionRecordDetailQuery {
            session_id: session(),
            record_id: "command:command-1".to_string(),
        })
        .expect("detail");
    assert_eq!(detail.counts.commands, 1);
    assert_eq!(detail.counts.tools, 1);
    assert_eq!(
        detail.error_reason_code.as_deref(),
        Some("execution_failed")
    );

    let missing = repository.record_detail(&ExecutionRecordDetailQuery {
        session_id: session(),
        record_id: "command:absent".to_string(),
    });
    assert_eq!(
        missing.expect_err("absent record"),
        EvidenceApplicationError::RecordNotFound
    );
}

#[test]
fn a_summary_counts_what_it_owns_and_declares_what_it_does_not() {
    let (_directory, _database, repository) = repository("evidence-summary");
    append(
        &repository,
        &command_completed("source-1", "command-1", "2026-01-01T00:00:05Z", 1),
    );
    append(
        &repository,
        &EvidenceEventBuilder::new(
            "source-verify",
            CorrelationBuilder::for_session(SESSION).build(),
            SafeEvidencePayload::VerificationCompleted {
                name: label("npm run test"),
                outcome:
                    crate::contexts::execution_observability::domain::VerificationOutcome::Failed,
                passed_count: Some(138),
                failed_count: Some(2),
            },
        )
        .with_status(ExecutionStatus::Failed)
        .with_occurred_at("2026-01-01T00:00:06Z")
        .build(),
    );

    let summary = repository
        .summary(&WorkspaceEvidenceSummaryQuery {
            session_id: session(),
            seat_id: None,
        })
        .expect("summary");

    assert_eq!(summary.failed_records, 2);
    assert_eq!(summary.verification_passed, 138);
    assert_eq!(summary.verification_failed, 2);
    // Logs, Shells, changes, review, and usage have no port here; each is declared unavailable
    // rather than counted as zero.
    assert_eq!(summary.unowned_sources.len(), 5);
    assert!(summary
        .unowned_sources
        .iter()
        .all(
            |source| source.coverage_state == EvidenceCoverageState::Unavailable
                && source.reason_code == reason_codes::SOURCE_NOT_OWNED
        ));
}

#[test]
fn a_bootstrap_watermark_advances_with_committed_events() {
    let (_directory, _database, repository) = repository("evidence-bootstrap");
    let empty = repository
        .subscription_bootstrap(&session())
        .expect("bootstrap");
    assert_eq!(empty.watermark_sequence, 0);

    append(
        &repository,
        &command_started("source-1", "command-1", "2026-01-01T00:00:00Z"),
    );
    let after = repository
        .subscription_bootstrap(&session())
        .expect("bootstrap");
    assert!(after.watermark_sequence > 0);
}

#[test]
fn events_that_are_not_execution_records_stay_in_the_journal_only() {
    let (_directory, database, repository) = repository("evidence-journal-only");
    append(
        &repository,
        &EvidenceEventBuilder::new(
            "source-shell",
            CorrelationBuilder::for_session(SESSION).build(),
            SafeEvidencePayload::ShellOpened {
                runtime_kind: CommandRuntimeKind::RemoteShell,
            },
        )
        .with_status(ExecutionStatus::Running)
        .build(),
    );

    let connection = database.connection().expect("connection");
    let events: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM execution_evidence_events",
            [],
            |row| row.get(0),
        )
        .expect("count");
    let records: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM execution_evidence_records",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(events, 1);
    assert_eq!(
        records, 0,
        "a Shell lifecycle event is evidence, not an execution record"
    );
}

#[test]
fn a_source_context_is_part_of_the_idempotency_key() {
    let (_directory, _database, repository) = repository("evidence-source-context");
    let base = command_started("source-1", "command-1", "2026-01-01T00:00:00Z");
    append(&repository, &base);

    // The same source event id from a different context is a different assertion, not a retry.
    let other = EvidenceEventBuilder::new(
        "source-1",
        CorrelationBuilder::for_session(SESSION)
            .with_run(RUN, TRACE)
            .with_command("command-2")
            .build(),
        SafeEvidencePayload::CommandStarted {
            runtime_kind: CommandRuntimeKind::Process,
            redacted_display: None,
            cwd_display: None,
        },
    )
    .with_source(EvidenceSourceContext::Workspaces)
    // Event ids are generated per attempt in production; the fixture assigns one explicitly so the
    // test exercises the source-context half of the idempotency key rather than an id collision.
    .with_event_id("event-workspaces-source-1")
    .with_status(ExecutionStatus::Running)
    .build();
    assert!(matches!(
        append(&repository, &other),
        EvidenceAppendOutcome::Appended { .. }
    ));
}

#[test]
fn a_stored_payload_round_trips_through_the_journal() {
    let (_directory, database, repository) = repository("evidence-payload-roundtrip");
    append(
        &repository,
        &command_started("source-1", "command-1", "2026-01-01T00:00:00Z"),
    );

    let connection = database.connection().expect("connection");
    let json: String = connection
        .query_row(
            "SELECT safe_payload_json FROM execution_evidence_events LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("payload");
    assert!(json.contains("command.started"));
    assert!(json.contains("npm test"));
}

/// Replay is a pure function of the journal.
///
/// The projection is disposable by design: if it is ever wrong it can be discarded and rebuilt
/// rather than patched, and the journal stays the only thing that has to be right. This is the
/// controlled repair that proves it.
#[test]
fn replaying_the_journal_reproduces_the_same_projection() {
    let (_directory, database, repository) = repository("evidence-replay");
    append(
        &repository,
        &command_started("source-1", "command-1", "2026-01-01T00:00:00Z"),
    );
    append(
        &repository,
        &command_completed("source-2", "command-1", "2026-01-01T00:00:05Z", 1),
    );
    append(
        &repository,
        &EvidenceEventBuilder::new(
            "source-tool",
            CorrelationBuilder::for_session(SESSION)
                .with_run(RUN, TRACE)
                .with_tool_call("tool-1")
                .build(),
            SafeEvidencePayload::ToolCompleted {
                tool_name: label("read_file"),
                outcome: EvidenceOutcome::Succeeded,
                duration_ms: Some(31),
            },
        )
        .with_status(ExecutionStatus::Succeeded)
        .with_occurred_at("2026-01-01T00:00:06Z")
        .build(),
    );

    let before = repository.list_records(&query(10)).expect("page").items;

    {
        let connection = database.connection().expect("connection");
        super::maintenance::reset_projections(&connection).expect("reset");
    }
    assert!(
        repository
            .list_records(&query(10))
            .expect("page")
            .items
            .is_empty(),
        "the reset removed the projection"
    );

    let replayed = repository.replay_projections(None).expect("replay");
    assert_eq!(replayed, 3, "every projecting event is replayed");

    let after = repository.list_records(&query(10)).expect("page").items;
    assert_eq!(before, after, "replay reproduces the projection exactly");
}

#[test]
fn replaying_twice_changes_nothing_and_appends_no_event() {
    let (_directory, database, repository) = repository("evidence-replay-idempotent");
    append(
        &repository,
        &command_completed("source-1", "command-1", "2026-01-01T00:00:05Z", 1),
    );

    let journal = |database: &NativeDatabase| -> i64 {
        database
            .connection()
            .expect("connection")
            .query_row(
                "SELECT COUNT(*) FROM execution_evidence_events",
                [],
                |row| row.get(0),
            )
            .expect("count")
    };
    let before = journal(&database);

    repository.replay_projections(None).expect("first replay");
    let first = repository.list_records(&query(10)).expect("page").items;
    repository.replay_projections(None).expect("second replay");
    let second = repository.list_records(&query(10)).expect("page").items;

    assert_eq!(first, second, "replay is deterministic");
    assert_eq!(
        before,
        journal(&database),
        "replay reads the journal and never writes to it"
    );
}

#[test]
fn retention_removes_expired_evidence_in_a_bounded_batch() {
    let (_directory, database, repository) = repository("evidence-retention");
    for index in 0..4 {
        append(
            &repository,
            &command_started(
                &format!("source-old-{index}"),
                &format!("command-old-{index}"),
                &format!("2025-01-0{}T00:00:00Z", index + 1),
            ),
        );
    }
    append(
        &repository,
        &command_started("source-new", "command-new", "2026-06-01T00:00:00Z"),
    );

    let outcome = repository
        .maintain_retention("2026-01-01T00:00:00Z", "2026-06-02T00:00:00Z")
        .expect("retention");
    assert_eq!(outcome.deleted_events, 4);

    let remaining: i64 = database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT COUNT(*) FROM execution_evidence_events",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(remaining, 1, "only evidence inside the window survives");

    let page = repository.list_records(&query(10)).expect("page");
    assert_eq!(
        page.items.len(),
        1,
        "a projection whose backing events all expired goes with them"
    );
    assert_eq!(page.coverage.state(), EvidenceCoverageState::Partial);
    assert!(page
        .coverage
        .reason_codes()
        .iter()
        .map(SafeReasonCode::as_str)
        .any(|code| code == reason_codes::RETENTION_EXPIRED));
    assert_eq!(
        page.coverage.oldest_available_at(),
        Some("2026-06-01T00:00:00Z"),
        "the oldest available boundary moves with the cutoff"
    );
}

/// The batch bound, at the bound.
///
/// The test above proves retention deletes; it uses four rows against a bound of five hundred, so
/// it has never once exercised the thing the bound is for. A pass that quietly deleted everything
/// in one statement would satisfy it, and would hold the write lock across an entire backlog —
/// which on a machine with a year of history is the whole database, unavailable, at startup.
///
/// Built from `RETENTION_BATCH` rather than from a literal beside it: a fixture with its own copy
/// of the number stops testing the boundary the moment somebody changes the real one, and goes on
/// passing while it does.
#[test]
fn retention_at_the_batch_bound_stops_at_the_bound_and_drains_across_passes() {
    let (_directory, database, repository) = repository("evidence-retention-capacity");
    let expired = super::maintenance::RETENTION_BATCH + 1;
    for index in 0..expired {
        append(
            &repository,
            &command_started(
                &format!("source-old-{index}"),
                &format!("command-old-{index}"),
                // Every one of them is inside the same expired day. Spreading them over distinct
                // days would let the batch boundary fall on a date change and pass for that reason
                // rather than because the limit held.
                "2025-01-01T00:00:00Z",
            ),
        );
    }

    let first = repository
        .maintain_retention("2026-01-01T00:00:00Z", "2026-06-02T00:00:00Z")
        .expect("first pass");
    assert_eq!(
        first.deleted_events,
        super::maintenance::RETENTION_BATCH,
        "one pass must stop at the bound rather than draining the backlog"
    );

    let second = repository
        .maintain_retention("2026-01-01T00:00:00Z", "2026-06-02T00:00:00Z")
        .expect("second pass");
    assert_eq!(
        second.deleted_events, 1,
        "the remainder goes on the next pass"
    );

    let remaining: i64 = database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT COUNT(*) FROM execution_evidence_events",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(
        remaining, 0,
        "and the backlog does drain, rather than stall"
    );
}

/// A record whose lifecycle straddles the cutoff keeps its projection: its newest event is still
/// inside the window, and dropping it would lose work the user can still act on.
#[test]
fn a_lifecycle_that_straddles_the_cutoff_is_retained() {
    let (_directory, _database, repository) = repository("evidence-retention-straddle");
    append(
        &repository,
        &command_started("source-start", "command-1", "2025-12-31T23:59:00Z"),
    );
    append(
        &repository,
        &command_completed("source-end", "command-1", "2026-01-02T00:00:00Z", 1),
    );

    repository
        .maintain_retention("2026-01-01T00:00:00Z", "2026-01-03T00:00:00Z")
        .expect("retention");

    let page = repository.list_records(&query(10)).expect("page");
    assert_eq!(page.items.len(), 1, "the straddling record survives");
    assert_eq!(page.items[0].status, ExecutionStatus::Failed);
}

// Retention must never turn into a full scan on a write path.
#[test]
fn the_retention_sweep_uses_its_index() {
    let (_directory, database, _repository) = repository("evidence-retention-plan");
    let connection = database.connection().expect("connection");
    let plan: String = connection
        .query_row(
            "EXPLAIN QUERY PLAN SELECT sequence FROM execution_evidence_events              WHERE occurred_at < '2026-01-01T00:00:00Z' ORDER BY occurred_at, sequence LIMIT 500",
            [],
            |row| row.get(3),
        )
        .expect("query plan");
    assert!(
        plan.contains("idx_execution_evidence_retention"),
        "expected the retention index, got: {plan}"
    );
}

/// Deleting expired projections must not read the journal once per candidate.
///
/// A record survives while any retained event still backs it, and the check is a correlation
/// lookup rather than a stored back-reference. That shape is right and it is also the shape that
/// degrades worst: three `OR`ed columns inside a `NOT EXISTS` is the classic way to make SQLite
/// give up on indexes and read a session's whole event history for every record it is considering.
/// Nothing about that is visible in a test with ten rows — it is visible in a long session, on a
/// user's machine, as a retention pass that holds the write lock for minutes.
///
/// Read off the plan rather than timed. A duration assertion on a shared runner measures the
/// runner; the plan is the property.
#[test]
fn expiring_projections_never_reads_the_whole_journal_per_record() {
    let (_directory, database, _repository) = repository("evidence-retention-projection-plan");
    let connection = database.connection().expect("connection");
    let plan: Vec<String> = {
        let mut statement = connection
            .prepare(
                "EXPLAIN QUERY PLAN DELETE FROM execution_evidence_records \
                 WHERE occurred_at < '2026-01-01T00:00:00Z' \
                   AND NOT EXISTS ( \
                     SELECT 1 FROM execution_evidence_events event \
                     WHERE event.session_id = execution_evidence_records.session_id \
                       AND ( \
                         event.command_id = SUBSTR(execution_evidence_records.record_id, 9) \
                         OR event.tool_call_id = SUBSTR(execution_evidence_records.record_id, 6) \
                         OR event.agent_id = execution_evidence_records.agent_id \
                       ) \
                   )",
            )
            .expect("prepare plan");
        let rows = statement
            .query_map([], |row| row.get::<_, String>(3))
            .expect("plan rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("plan");
        rows
    };
    let plan = plan.join(" | ");

    // Matched on `SCAN` rather than on a table name, because SQLite reports the subquery under the
    // alias `event` — the first version of this test asked for `SCAN execution_evidence_events`,
    // which never appears under any plan and so passed while checking nothing.
    assert!(
        !plan.contains("SCAN "),
        "something in the retention delete reads a table end to end: {plan}"
    );
    // The two halves, named. Without these the `SCAN` assertion above passes on an empty plan, on
    // a rewritten delete that dropped the survivor check, and on a renamed index.
    assert!(
        plan.contains("CORRELATED SCALAR SUBQUERY"),
        "the per-candidate survivor check is gone from the plan: {plan}"
    );
    assert!(
        plan.contains("USING INDEX idx_execution_evidence_session_sequence"),
        "the survivor check no longer reaches the journal through its session index: {plan}"
    );
    assert!(
        plan.contains("USING INDEX idx_evidence_records_retention"),
        "the outer delete no longer seeks the expired window: {plan}"
    );
}

/// The staleness check against a real store, not a flag.
///
/// It has to distinguish three states that all look alike from outside: an empty store, a store
/// whose projection is current, and one whose projection is behind. Only the third may trigger a
/// rebuild — and a store holding only non-projecting events (a run start, a usage observation)
/// must read as current, or every launch would rebuild forever chasing a row that never appears.
#[test]
fn projection_staleness_distinguishes_current_from_behind() {
    let (_directory, _database, repository) = repository("evidence-projection-staleness");

    assert!(
        !repository.projection_is_stale().expect("empty store"),
        "an empty store has nothing to rebuild"
    );

    repository
        .append(
            &command_started("source-1", "command-1", "2026-08-22T10:00:00Z"),
            "fingerprint-1",
            "2026-08-22T10:00:00Z",
        )
        .expect("append");
    assert!(
        !repository.projection_is_stale().expect("after append"),
        "the projection is written in the same transaction, so it is never behind after an append"
    );

    super::maintenance::reset_projections(&repository.pooled_connection().expect("connection"))
        .expect("reset");
    assert!(
        repository.projection_is_stale().expect("after reset"),
        "a projection cleared behind a non-empty journal is exactly the case a rebuild repairs"
    );

    let replayed = repository.replay_projections(None).expect("replay");
    assert_eq!(replayed, 1);
    assert!(!repository.projection_is_stale().expect("after replay"));
}

/// One tool invocation, on a named seat, with a chosen fidelity.
fn tool_started(
    source_event_id: &str,
    tool_call_id: &str,
    tool_name: &str,
    seat_id: &str,
    fidelity: ExecutionFidelity,
    occurred_at: &str,
) -> ExecutionEvidenceEvent {
    EvidenceEventBuilder::new(
        source_event_id,
        CorrelationBuilder::for_session(SESSION)
            .with_run(RUN, TRACE)
            .with_seat(seat_id)
            .with_tool_call(tool_call_id)
            .build(),
        SafeEvidencePayload::ToolStarted {
            tool_name: label(tool_name),
        },
    )
    .with_status(ExecutionStatus::Running)
    .with_fidelity(fidelity)
    .with_occurred_at(occurred_at)
    .build()
}

fn delegation_started(
    source_event_id: &str,
    agent_id: &str,
    run_id: &str,
    occurred_at: &str,
) -> ExecutionEvidenceEvent {
    EvidenceEventBuilder::new(
        source_event_id,
        CorrelationBuilder::for_session(SESSION)
            .with_run(run_id, TRACE)
            .with_agent(agent_id)
            .build(),
        SafeEvidencePayload::AgentDelegated { attempt: Some(1) },
    )
    .with_status(ExecutionStatus::Running)
    .with_occurred_at(occurred_at)
    .build()
}

fn verification_completed(
    source_event_id: &str,
    name: &str,
    occurred_at: &str,
) -> ExecutionEvidenceEvent {
    EvidenceEventBuilder::new(
        source_event_id,
        CorrelationBuilder::for_session(SESSION)
            .with_run(RUN, TRACE)
            .build(),
        SafeEvidencePayload::VerificationCompleted {
            name: label(name),
            outcome: VerificationOutcome::Passed,
            passed_count: Some(138),
            failed_count: Some(0),
        },
    )
    .with_status(ExecutionStatus::Succeeded)
    .with_occurred_at(occurred_at)
    .build()
}

/// One of each record kind, so a filter that selected the wrong rows would show it.
fn seed_every_kind(repository: &SqliteEvidenceRepository) {
    append(
        repository,
        &command_started("command-1", "cmd-1", "2026-08-22T10:00:00Z"),
    );
    append(
        repository,
        &tool_started(
            "tool-1",
            "call-1",
            "read_file",
            "seat-1",
            ExecutionFidelity::Proxied,
            "2026-08-22T10:01:00Z",
        ),
    );
    append(
        repository,
        &delegation_started("delegation-1", "agent-1", RUN, "2026-08-22T10:02:00Z"),
    );
    append(
        repository,
        &verification_completed("verification-1", "cargo test", "2026-08-22T10:03:00Z"),
    );
}

fn kinds_in(repository: &SqliteEvidenceRepository, query: ExecutionRecordQuery) -> Vec<String> {
    repository
        .list_records(&query)
        .expect("page")
        .items
        .iter()
        .map(|item| item.kind.as_str().to_string())
        .collect()
}

/// Terminal History reads the record query the journal already exposes.
///
/// Every dimension the console filters by is asserted against stored rows here, because the
/// alternative to proving it is building a second query beside this one — and two queries over one
/// projection disagree the first time a filter is added to only one of them.
#[test]
fn one_record_query_answers_every_kind_status_scope_and_search() {
    let (_directory, _database, repository) = repository("evidence-record-query");
    seed_every_kind(&repository);

    // Every kind is reachable, and each filter selects only its own.
    assert_eq!(kinds_in(&repository, query(50)).len(), 4);
    for kind in [
        ExecutionRecordKind::Command,
        ExecutionRecordKind::Tool,
        ExecutionRecordKind::Delegation,
        ExecutionRecordKind::Verification,
    ] {
        let mut scoped = query(50);
        scoped.filters.kinds = vec![kind];
        assert_eq!(
            kinds_in(&repository, scoped),
            vec![kind.as_str().to_string()],
            "the {} filter selected other kinds",
            kind.as_str()
        );
    }
}

#[test]
fn a_status_filter_selects_only_the_records_in_that_state() {
    let (_directory, _database, repository) = repository("evidence-record-status");
    seed_every_kind(&repository);

    let mut running = query(50);
    running.filters.statuses = vec![ExecutionStatus::Running];
    assert_eq!(kinds_in(&repository, running).len(), 3);

    let mut succeeded = query(50);
    succeeded.filters.statuses = vec![ExecutionStatus::Succeeded];
    assert_eq!(kinds_in(&repository, succeeded), vec!["verification"]);
}

/// Fidelity is how directly the runtime observed the work, and it is never upgraded by a reader.
/// A filter that ignored it would let a proxied row answer a question about native evidence.
#[test]
fn a_fidelity_filter_selects_only_records_observed_that_way() {
    let (_directory, _database, repository) = repository("evidence-record-fidelity");
    seed_every_kind(&repository);

    let mut proxied = query(50);
    proxied.filters.fidelities = vec![ExecutionFidelity::Proxied];
    assert_eq!(kinds_in(&repository, proxied), vec!["tool"]);

    let mut native = query(50);
    native.filters.fidelities = vec![ExecutionFidelity::Native];
    assert_eq!(kinds_in(&repository, native).len(), 3);
}

#[test]
fn a_seat_scope_narrows_to_one_participant_and_a_run_scope_to_one_run() {
    let (_directory, _database, repository) = repository("evidence-record-scope");
    seed_every_kind(&repository);
    let other_run = "7a2c3d4e-5f6a-4b7c-8d9e-0f1a2b3c4d5e";
    append(
        &repository,
        &delegation_started("delegation-2", "agent-2", other_run, "2026-08-22T10:04:00Z"),
    );

    let mut seated = query(50);
    seated.scope.seat_id = Some(EvidenceSeatId::parse("seat-1").expect("seat"));
    assert_eq!(kinds_in(&repository, seated), vec!["tool"]);

    let mut in_run = query(50);
    in_run.scope.run_id = Some(other_run.to_string());
    assert_eq!(kinds_in(&repository, in_run), vec!["delegation"]);
}

/// Search reads the redacted display fields the projection already holds and nothing else.
///
/// Scanning the journal payload would put raw producer content back into a query path that the
/// whole design exists to keep it out of.
#[test]
fn a_bounded_search_matches_only_redacted_projection_fields() {
    let (_directory, _database, repository) = repository("evidence-record-search");
    seed_every_kind(&repository);

    for (term, expected) in [
        ("npm", vec!["command"]),
        // Snake case on purpose: the pattern escapes `_` so a term cannot smuggle in a wildcard,
        // and an escape that the query never declares turns every tool name into a non-match.
        ("read_file", vec!["tool"]),
        // A wildcard supplied by the reader stays a literal. Both of these would match
        // `read_file` if `_` and `%` were still wildcards, and neither occurs in it verbatim.
        ("rea_", Vec::new()),
        ("read%file", Vec::new()),
        ("%", Vec::new()),
        ("cargo", vec!["verification"]),
        ("nothing-matches-this", Vec::new()),
    ] {
        let mut searched = query(50);
        searched.filters.search = Some(term.to_string());
        assert_eq!(
            kinds_in(&repository, searched),
            expected,
            "search for {term}"
        );
    }

    // A delegation carries no display text, so it is never a search hit — rather than matching
    // everything because the query had nothing to compare against.
    let mut blank = query(50);
    blank.filters.search = Some("   ".to_string());
    assert_eq!(kinds_in(&repository, blank).len(), 4);
}

/// A page stays newest-first and stable across a cursor, with each row appearing exactly once.
#[test]
fn record_pages_are_newest_first_and_never_repeat_a_row() {
    let (_directory, _database, repository) = repository("evidence-record-paging");
    seed_every_kind(&repository);

    let first = repository.list_records(&query(2)).expect("first page");
    assert_eq!(first.items.len(), 2);
    assert_eq!(
        first
            .items
            .iter()
            .map(|item| item.occurred_at.clone())
            .collect::<Vec<_>>(),
        vec![
            "2026-08-22T10:03:00Z".to_string(),
            "2026-08-22T10:02:00Z".to_string()
        ]
    );

    let mut next = query(2);
    next.cursor = first.next_cursor.clone();
    let second = repository.list_records(&next).expect("second page");

    let ids = first
        .items
        .iter()
        .chain(second.items.iter())
        .map(|item| item.record_id.clone())
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 4);
    assert_eq!(
        ids.iter().collect::<std::collections::BTreeSet<_>>().len(),
        4,
        "a row appeared on two pages"
    );
    assert_eq!(second.coverage.state(), EvidenceCoverageState::Complete);
}
