//! What the report aggregates count, and what they refuse to count.
//!
//! Against a real SQLite rather than a double, because everything under test here is SQL: a grouped
//! count, a conditional sum, an ordered offset. A double would only prove that the Rust around the
//! statements is wired up, which is the part that was never in doubt.

use super::{report_aggregate, SqliteEvidenceRepository};
use crate::contexts::execution_observability::application::evidence::ports::EvidenceRepositoryPort;
use crate::contexts::execution_observability::application::evidence::report_models::{
    failure_codes, EvidenceReportQuery, MAX_EVIDENCE_TOOL_ROWS,
};
use crate::contexts::execution_observability::domain::evidence::builders::{
    label, CorrelationBuilder, EvidenceEventBuilder,
};
use crate::contexts::execution_observability::domain::evidence::payload::EvidenceOutcome;
use crate::contexts::execution_observability::domain::evidence::safety::RedactedCommandDisplay;
use crate::contexts::execution_observability::domain::{
    CommandRuntimeKind, EvidenceSessionId, ExecutionEvidenceEvent, ExecutionStatus,
    OutputAvailability, SafeEvidencePayload, VerificationOutcome,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

const SESSION: &str = "session-1";
const RUN: &str = "6f1b2c3d-4e5f-4a6b-8c9d-0e1f2a3b4c5d";
const OTHER_RUN: &str = "7a2c3d4e-5f60-4b7c-9d0e-1f2a3b4c5d6e";
const TRACE: &str = "0af7651916cd43dd8448eb211c80319c";

fn repository(name: &str) -> (TempDirectory, SqliteEvidenceRepository) {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let repository = SqliteEvidenceRepository::new(database);
    (directory, repository)
}

fn query() -> EvidenceReportQuery {
    EvidenceReportQuery::new(EvidenceSessionId::parse(SESSION).expect("session"))
}

fn append(repository: &SqliteEvidenceRepository, event: &ExecutionEvidenceEvent) {
    repository
        .append(
            event,
            &event.canonical_fingerprint(),
            "2026-01-01T00:00:00Z",
        )
        .expect("append");
}

fn tool_started(
    source: &str,
    call_id: &str,
    tool: &str,
    occurred_at: &str,
) -> ExecutionEvidenceEvent {
    EvidenceEventBuilder::new(
        source,
        CorrelationBuilder::for_session(SESSION)
            .with_run(RUN, TRACE)
            .with_tool_call(call_id)
            .build(),
        SafeEvidencePayload::ToolStarted {
            tool_name: label(tool),
        },
    )
    .with_status(ExecutionStatus::Running)
    .with_occurred_at(occurred_at)
    .build()
}

fn tool_completed(
    source: &str,
    call_id: &str,
    tool: &str,
    occurred_at: &str,
    outcome: EvidenceOutcome,
    duration_ms: Option<u64>,
) -> ExecutionEvidenceEvent {
    let status = match outcome {
        EvidenceOutcome::Succeeded => ExecutionStatus::Succeeded,
        EvidenceOutcome::Failed => ExecutionStatus::Failed,
        _ => ExecutionStatus::Cancelled,
    };
    EvidenceEventBuilder::new(
        source,
        CorrelationBuilder::for_session(SESSION)
            .with_run(RUN, TRACE)
            .with_tool_call(call_id)
            .build(),
        SafeEvidencePayload::ToolCompleted {
            tool_name: label(tool),
            outcome,
            duration_ms,
        },
    )
    .with_status(status)
    .with_occurred_at(occurred_at)
    .build()
}

/// One tool call, start to finish.
fn tool_call(
    repository: &SqliteEvidenceRepository,
    index: usize,
    tool: &str,
    outcome: EvidenceOutcome,
    duration_ms: Option<u64>,
) {
    let call_id = format!("call-{index}");
    append(
        repository,
        &tool_started(
            &format!("tool-start-{index}"),
            &call_id,
            tool,
            &format!("2026-01-01T00:{:02}:00Z", index % 60),
        ),
    );
    append(
        repository,
        &tool_completed(
            &format!("tool-end-{index}"),
            &call_id,
            tool,
            &format!("2026-01-01T00:{:02}:30Z", index % 60),
            outcome,
            duration_ms,
        ),
    );
}

fn command_completed(
    source: &str,
    command_id: &str,
    exit_code: Option<i32>,
    signal: Option<&str>,
) -> ExecutionEvidenceEvent {
    EvidenceEventBuilder::new(
        source,
        CorrelationBuilder::for_session(SESSION)
            .with_run(RUN, TRACE)
            .with_command(command_id)
            .build(),
        SafeEvidencePayload::CommandCompleted {
            outcome: EvidenceOutcome::Failed,
            duration_ms: Some(1_000),
            exit_code,
            signal: signal.map(label),
            output_availability: OutputAvailability::Merged,
            output_truncated: false,
        },
    )
    .with_status(ExecutionStatus::Failed)
    .with_occurred_at("2026-01-01T00:10:00Z")
    .build()
}

fn command_started(source: &str, command_id: &str) -> ExecutionEvidenceEvent {
    EvidenceEventBuilder::new(
        source,
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
    .with_occurred_at("2026-01-01T00:09:00Z")
    .build()
}

fn verification(
    source: &str,
    name: &str,
    outcome: VerificationOutcome,
    passed: Option<u32>,
    failed: Option<u32>,
) -> ExecutionEvidenceEvent {
    let status = match outcome {
        VerificationOutcome::Failed => ExecutionStatus::Failed,
        _ => ExecutionStatus::Succeeded,
    };
    EvidenceEventBuilder::new(
        source,
        CorrelationBuilder::for_session(SESSION)
            .with_run(RUN, TRACE)
            .with_operation(source)
            .build(),
        SafeEvidencePayload::VerificationCompleted {
            name: label(name),
            outcome,
            passed_count: passed,
            failed_count: failed,
        },
    )
    .with_status(status)
    .with_occurred_at("2026-01-01T00:20:00Z")
    .build()
}

// ---------------------------------------------------------------------------------------------
// Tools
// ---------------------------------------------------------------------------------------------

#[test]
fn tool_rows_count_invocations_and_failures_per_tool() {
    let (_directory, repository) = repository("report-tools");
    tool_call(
        &repository,
        1,
        "read_file",
        EvidenceOutcome::Succeeded,
        Some(100),
    );
    tool_call(
        &repository,
        2,
        "read_file",
        EvidenceOutcome::Failed,
        Some(200),
    );
    tool_call(
        &repository,
        3,
        "write_file",
        EvidenceOutcome::Succeeded,
        Some(50),
    );

    let aggregate = repository.report_aggregate(&query()).expect("aggregate");

    // Heaviest first, so a reader sees what the session actually spent its calls on.
    assert_eq!(aggregate.tools.len(), 2);
    assert_eq!(aggregate.tools[0].tool_name, "read_file");
    assert_eq!(aggregate.tools[0].invocations, 2);
    assert_eq!(aggregate.tools[0].failures, 1);
    assert_eq!(aggregate.tools[0].duration_ms, Some(300));
}

/// A group with one unmeasured call reports no duration at all.
#[test]
fn a_tool_group_with_an_unmeasured_call_reports_no_total_duration() {
    let (_directory, repository) = repository("report-tool-unmeasured");
    tool_call(
        &repository,
        1,
        "read_file",
        EvidenceOutcome::Succeeded,
        Some(100),
    );
    tool_call(
        &repository,
        2,
        "read_file",
        EvidenceOutcome::Succeeded,
        None,
    );

    let aggregate = repository.report_aggregate(&query()).expect("aggregate");

    // 100 would be the sum of what was measured, presented as the sum of what happened. The
    // shortfall is unrecoverable from the answer, so there is no answer.
    assert_eq!(aggregate.tools[0].invocations, 2);
    assert_eq!(aggregate.tools[0].duration_ms, None);
}

#[test]
fn the_tool_tail_is_cut_and_the_aggregate_says_so() {
    let (_directory, repository) = repository("report-tool-tail");
    for index in 0..MAX_EVIDENCE_TOOL_ROWS + 3 {
        tool_call(
            &repository,
            index,
            &format!("tool_{index:02}"),
            EvidenceOutcome::Succeeded,
            Some(10),
        );
    }

    let aggregate = repository.report_aggregate(&query()).expect("aggregate");

    assert_eq!(aggregate.tools.len(), MAX_EVIDENCE_TOOL_ROWS);
    // A truncated list that claimed completeness is the same false claim as a zero for a missing
    // measurement.
    assert!(aggregate.incomplete);
}

// ---------------------------------------------------------------------------------------------
// Commands and verifications
// ---------------------------------------------------------------------------------------------

#[test]
fn a_running_command_leaves_the_command_total_duration_absent() {
    let (_directory, repository) = repository("report-command-running");
    append(&repository, &command_started("command-open", "command-1"));

    let aggregate = repository.report_aggregate(&query()).expect("aggregate");

    assert_eq!(aggregate.commands.total, 1);
    assert_eq!(aggregate.commands.running, 1);
    // The session is not over, and a total that silently excluded the open command would read as
    // though it were.
    assert_eq!(aggregate.commands.duration_ms, None);
}

#[test]
fn verification_counts_checks_and_counts_skips_by_record() {
    let (_directory, repository) = repository("report-verification");
    append(
        &repository,
        &verification(
            "check-1",
            "unit",
            VerificationOutcome::Passed,
            Some(10),
            Some(0),
        ),
    );
    append(
        &repository,
        &verification(
            "check-2",
            "e2e",
            VerificationOutcome::Failed,
            Some(1),
            Some(2),
        ),
    );
    append(
        &repository,
        &verification("check-3", "lint", VerificationOutcome::Skipped, None, None),
    );

    let aggregate = repository.report_aggregate(&query()).expect("aggregate");

    assert_eq!(aggregate.verification.passed, 11);
    assert_eq!(aggregate.verification.failed, 2);
    // A skipped verification ran no assertions, so a record is the only unit it can be counted in.
    assert_eq!(aggregate.verification.skipped, 1);
}

// ---------------------------------------------------------------------------------------------
// Failures
// ---------------------------------------------------------------------------------------------

#[test]
fn a_signalled_command_is_a_different_failure_from_a_non_zero_exit() {
    let (_directory, repository) = repository("report-failure-codes");
    append(
        &repository,
        &command_completed("exit-1", "command-1", Some(1), None),
    );
    append(
        &repository,
        &command_completed("signal-1", "command-2", None, Some("SIGKILL")),
    );
    tool_call(
        &repository,
        1,
        "read_file",
        EvidenceOutcome::Failed,
        Some(10),
    );

    let aggregate = repository.report_aggregate(&query()).expect("aggregate");

    let codes: Vec<&str> = aggregate
        .failures
        .iter()
        .map(|row| row.reason_code.as_str())
        .collect();
    // A non-zero exit is the program's own verdict; a signal is the platform killing it, and a
    // reader chases the two to different places.
    assert!(codes.contains(&failure_codes::COMMAND_EXIT));
    assert!(codes.contains(&failure_codes::COMMAND_SIGNAL));
    assert!(codes.contains(&failure_codes::TOOL));
}

#[test]
fn a_failure_row_carries_no_tool_name_or_command_text() {
    let (_directory, repository) = repository("report-failure-text");
    append(
        &repository,
        &command_completed("exit-1", "command-1", Some(1), None),
    );

    let aggregate = repository.report_aggregate(&query()).expect("aggregate");

    // The projection holds `npm test` as a redacted display, and grouping by it would put producer
    // text into a report somebody quotes.
    for row in &aggregate.failures {
        assert!(
            !row.reason_code.contains(' '),
            "{} reads as prose",
            row.reason_code
        );
        assert!(!row.reason_code.contains("npm"));
    }
}

// ---------------------------------------------------------------------------------------------
// Scope
// ---------------------------------------------------------------------------------------------

#[test]
fn a_run_filter_excludes_records_from_other_runs() {
    let (_directory, repository) = repository("report-run-scope");
    tool_call(
        &repository,
        1,
        "read_file",
        EvidenceOutcome::Succeeded,
        Some(10),
    );

    let scoped = repository
        .report_aggregate(&EvidenceReportQuery {
            run_ids: vec![OTHER_RUN.to_string()],
            ..query()
        })
        .expect("aggregate");

    assert!(scoped.tools.is_empty());
}

#[test]
fn a_time_range_excludes_records_outside_it() {
    let (_directory, repository) = repository("report-time-scope");
    tool_call(
        &repository,
        1,
        "read_file",
        EvidenceOutcome::Succeeded,
        Some(10),
    );

    let before = repository
        .report_aggregate(&EvidenceReportQuery {
            to: Some("2025-12-31T00:00:00Z".to_string()),
            ..query()
        })
        .expect("aggregate");

    assert!(before.tools.is_empty());
}

// ---------------------------------------------------------------------------------------------
// Latency
// ---------------------------------------------------------------------------------------------

#[test]
fn percentiles_come_from_the_measured_durations() {
    let (_directory, repository) = repository("report-latency");
    for (index, duration) in [100_u64, 200, 300, 400, 500].into_iter().enumerate() {
        tool_call(
            &repository,
            index,
            "read_file",
            EvidenceOutcome::Succeeded,
            Some(duration),
        );
    }

    let latency = repository.report_latency(&query()).expect("latency");

    // Nearest-rank over five samples: p50 is the third, p95 clamps to the last.
    assert_eq!(latency.p50_ms, Some(300));
    assert_eq!(latency.p95_ms, Some(500));
    assert_eq!(latency.slowest_record_duration_ms, Some(500));
}

#[test]
fn a_session_where_nothing_finished_has_no_percentiles() {
    let (_directory, repository) = repository("report-latency-empty");
    append(&repository, &command_started("command-open", "command-1"));

    let latency = repository.report_latency(&query()).expect("latency");

    // Absent, not zero: a p50 of zero reports a session where every call returned instantly.
    assert_eq!(latency.p50_ms, None);
    assert_eq!(latency.p95_ms, None);
    assert_eq!(latency.slowest_record_duration_ms, None);
}

#[test]
fn a_single_measured_record_is_its_own_p50_and_p95() {
    let (_directory, repository) = repository("report-latency-single");
    tool_call(
        &repository,
        1,
        "read_file",
        EvidenceOutcome::Succeeded,
        Some(700),
    );

    let latency = repository.report_latency(&query()).expect("latency");

    // The clamp is what makes this work: an unclamped p95 offset would land past the last row and
    // return nothing, which reads as "no measurement" for a session that measured one.
    assert_eq!(latency.p50_ms, Some(700));
    assert_eq!(latency.p95_ms, Some(700));
}

// ---------------------------------------------------------------------------------------------
// Plans
// ---------------------------------------------------------------------------------------------

/// The aggregate runs on a tab open, over a table that grows with every recorded action.
#[test]
fn the_aggregate_scope_is_served_by_the_session_index() {
    let (directory, _repository) = repository("report-plan");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("reopen");
    let connection = database.connection().expect("connection");
    let plan: String = connection
        .query_row(
            "EXPLAIN QUERY PLAN SELECT tool_name, COUNT(*) FROM execution_evidence_records \
             WHERE session_id = 'session-1' AND record_kind = 'tool' GROUP BY tool_name",
            [],
            |row| row.get(3),
        )
        .expect("query plan");
    // Which of the two session-leading indexes SQLite picks is its business — both start with
    // `session_id`, and the planner is free to prefer either. What must not appear is `SCAN`, which
    // is the difference between a seek into one session and a pass over every record ever recorded.
    assert!(
        plan.contains("USING INDEX") && !plan.contains("SCAN execution_evidence_records"),
        "expected an indexed seek, got: {plan}"
    );
}

/// A direct call to the private helpers, so a future refactor that moves the SQL keeps a caller.
#[test]
fn the_helpers_answer_an_empty_store_without_error() {
    let (directory, _repository) = repository("report-empty");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("reopen");
    let connection = database.connection().expect("connection");

    let aggregate =
        report_aggregate::report_aggregate(&connection, &query(), 0).expect("aggregate");
    let latency = report_aggregate::report_latency(&connection, &query(), 0).expect("latency");

    assert!(aggregate.tools.is_empty());
    assert_eq!(aggregate.commands.total, 0);
    assert_eq!(latency.p50_ms, None);
}
