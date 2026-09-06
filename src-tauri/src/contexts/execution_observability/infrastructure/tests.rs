use super::SqliteExecutionTimelineRepository;
use crate::contexts::execution_observability::application::ExecutionTelemetryPort;
use crate::contexts::execution_observability::domain::{
    CapturePolicy, ExecutionContext, ExecutionEvent, ExecutionFidelity, ExecutionRun,
    ExecutionRunId, ExecutionSource, ExecutionSpan, ExecutionStatus, ObservabilitySettings,
    PageRequest, SafeAttributes, SpanId, TraceId,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use rusqlite::params;

fn repository(name: &str) -> (TempDirectory, SqliteExecutionTimelineRepository) {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).unwrap();
    (directory, SqliteExecutionTimelineRepository::new(database))
}

fn context(index: u64) -> ExecutionContext {
    ExecutionContext {
        run_id: ExecutionRunId::parse(format!("00000000-0000-4000-8000-{index:012x}")).unwrap(),
        trace_id: TraceId::parse(format!("{index:032x}")).unwrap(),
        span_id: SpanId::parse(format!("{index:016x}")).unwrap(),
        capture_policy: CapturePolicy::MetadataOnly,
        sampling_per_million: 1_000_000,
        mcp_relay_enabled: false,
    }
}

fn run(index: u64, started_at: &str) -> ExecutionRun {
    ExecutionRun {
        context: context(index),
        source: ExecutionSource::Desktop,
        status: ExecutionStatus::Running,
        started_at: started_at.to_string(),
        ended_at: None,
        error_classification: None,
        session_id: Some("session-safe".to_string()),
        user_message_id: Some(format!("message-{index}")),
        assistant_message_id: None,
        operation_id: Some(format!("operation-{index}")),
        agent_id: Some("codex-cli".to_string()),
        provider_session_id: None,
        attributes: SafeAttributes::default(),
        links: Vec::new(),
    }
}

fn span(run: &ExecutionRun) -> ExecutionSpan {
    ExecutionSpan {
        context: run.context.clone(),
        parent_span_id: None,
        name: "vanehub.task.execute".to_string(),
        status: ExecutionStatus::Running,
        fidelity: ExecutionFidelity::Native,
        started_at: run.started_at.clone(),
        ended_at: None,
        error_classification: None,
        attributes: SafeAttributes::default(),
        links: Vec::new(),
    }
}

#[test]
fn round_trips_terminal_timeline_and_deduplicates_events() {
    let (_directory, repository) = repository("execution-timeline-roundtrip");
    let run = run(1, "2026-07-23T00:00:00Z");
    let span = span(&run);
    let event = ExecutionEvent {
        run_id: run.context.run_id.clone(),
        span_id: span.context.span_id.clone(),
        sequence: 1,
        name: "process.spawned".to_string(),
        timestamp: "2026-07-23T00:00:01Z".to_string(),
        attributes: SafeAttributes::default(),
    };

    repository.start_run(&run).unwrap();
    repository.start_run(&run).unwrap();
    repository.start_span(&span).unwrap();
    repository.record_event(&event).unwrap();
    repository.record_event(&event).unwrap();
    repository
        .finish_span(
            &run.context.run_id,
            &span.context.span_id,
            ExecutionStatus::Succeeded,
            "2026-07-23T00:00:02Z",
            None,
        )
        .unwrap();
    repository
        .finish_run(
            &run.context.run_id,
            ExecutionStatus::Succeeded,
            "2026-07-23T00:00:02Z",
            None,
        )
        .unwrap();

    let timeline = repository.timeline(&run.context.run_id).unwrap().unwrap();
    assert_eq!(timeline.run.status, ExecutionStatus::Succeeded);
    assert_eq!(timeline.spans.len(), 1);
    assert_eq!(timeline.spans[0].status, ExecutionStatus::Succeeded);
    assert_eq!(timeline.events, vec![event]);
}

#[test]
fn paginates_by_stable_time_and_run_cursor() {
    let (_directory, repository) = repository("execution-timeline-pagination");
    for index in 1..=3 {
        repository
            .start_run(&run(index, &format!("2026-07-23T00:00:0{index}Z")))
            .unwrap();
    }

    let first = repository
        .list_runs(&PageRequest::new(2, None).unwrap(), Some("session-safe"))
        .unwrap();
    assert_eq!(first.items.len(), 2);
    assert!(first.next_page_token.is_some());
    let second = repository
        .list_runs(
            &PageRequest::new(2, first.next_page_token).unwrap(),
            Some("session-safe"),
        )
        .unwrap();
    assert_eq!(second.items.len(), 1);
    assert!(second.next_page_token.is_none());
}

#[test]
fn preserves_open_spans_after_a_simulated_crash() {
    let (_directory, repository) = repository("execution-timeline-open-span");
    let run = run(4, "2026-07-23T00:00:00Z");
    repository.start_run(&run).unwrap();
    repository.start_span(&span(&run)).unwrap();

    let timeline = repository.timeline(&run.context.run_id).unwrap().unwrap();
    assert_eq!(timeline.run.status, ExecutionStatus::Running);
    assert_eq!(timeline.spans[0].status, ExecutionStatus::Running);
    assert!(timeline.spans[0].ended_at.is_none());
}

#[test]
fn retention_runs_on_schedule_instead_of_per_event() {
    let (_directory, repository) = repository("execution-timeline-retention");
    repository
        .start_run(&run(5, "2026-01-01T00:00:00Z"))
        .unwrap();
    repository
        .start_run(&run(6, "2026-07-22T00:00:00Z"))
        .unwrap();

    let first = repository
        .maintain_retention("2026-07-23T00:00:00Z", 30)
        .unwrap();
    assert!(first.ran);
    assert_eq!(first.deleted_runs, 1);
    let repeated = repository
        .maintain_retention("2026-07-23T01:00:00Z", 30)
        .unwrap();
    assert!(!repeated.ran);
    assert_eq!(repeated.deleted_runs, 0);
    assert!(repository.timeline(&context(5).run_id).unwrap().is_none());
    assert!(repository.timeline(&context(6).run_id).unwrap().is_some());
}

#[test]
fn settings_use_safe_defaults_and_round_trip_valid_updates() {
    let (_directory, repository) = repository("execution-observability-settings");
    assert_eq!(
        repository.load_settings().unwrap(),
        ObservabilitySettings::default()
    );
    let settings = ObservabilitySettings {
        otlp_enabled: true,
        otlp_endpoint: Some("https://collector.example.com/v1/traces".to_string()),
        sampling_ratio: 0.25,
        retention_days: 14,
        capture_policy: CapturePolicy::RedactedContent,
        mcp_relay_enabled: true,
        ..ObservabilitySettings::default()
    };

    repository
        .update_settings(&settings, "2026-07-23T00:00:00Z")
        .unwrap();

    assert_eq!(repository.load_settings().unwrap(), settings);
}

#[test]
fn invalid_settings_updates_leave_the_previous_snapshot_unchanged() {
    let (_directory, repository) = repository("execution-observability-settings-rollback");
    let previous = repository.load_settings().unwrap();
    let mut invalid = previous.clone();
    invalid.retention_days = 91;

    assert!(repository
        .update_settings(&invalid, "2026-07-23T00:00:00Z")
        .is_err());
    assert_eq!(repository.load_settings().unwrap(), previous);
}

/// The statuses a terminal record must survive being restarted from.
///
/// All of them, not a representative one: the conflict branch resolves status with a single
/// expression, so a test exercising only `succeeded` would pass against a fix that special-cased
/// success while still letting a failure be reopened.
const TERMINAL_STATUSES: [ExecutionStatus; 4] = [
    ExecutionStatus::Succeeded,
    ExecutionStatus::Failed,
    ExecutionStatus::Cancelled,
    ExecutionStatus::Incomplete,
];

/// A replayed start must not undo a finish.
///
/// Retries, duplicate deliveries, and late events are ordinary in a telemetry path, and a producer
/// cannot know whether a finish already landed. "Callers should not do that" is not an invariant:
/// either the store refuses the regression or it does not have one.
#[test]
fn a_replayed_start_cannot_reopen_a_terminal_run() {
    for (index, terminal) in TERMINAL_STATUSES.into_iter().enumerate() {
        let (_directory, store) = repository(&format!("execution-late-start-run-{index}"));
        // Offset past zero: an all-zero UUID is not a valid run id.
        let execution = run(index as u64 + 20, "2026-07-23T00:00:00Z");
        store.start_run(&execution).unwrap();
        store
            .finish_run(
                &execution.context.run_id,
                terminal,
                "2026-07-23T00:00:05Z",
                Some("classified"),
            )
            .unwrap();

        store.start_run(&execution).unwrap();

        let timeline = store
            .timeline(&execution.context.run_id)
            .unwrap()
            .expect("timeline");
        assert_eq!(
            timeline.run.status, terminal,
            "a {terminal:?} run was reopened by a replayed start"
        );
        assert_eq!(
            timeline.run.ended_at.as_deref(),
            Some("2026-07-23T00:00:05Z")
        );
    }
}

#[test]
fn a_replayed_start_cannot_reopen_a_terminal_span() {
    for (index, terminal) in TERMINAL_STATUSES.into_iter().enumerate() {
        let (_directory, store) = repository(&format!("execution-late-start-span-{index}"));
        // Offset past zero: an all-zero UUID is not a valid run id.
        let execution = run(index as u64 + 20, "2026-07-23T00:00:00Z");
        let observed = span(&execution);
        store.start_run(&execution).unwrap();
        store.start_span(&observed).unwrap();
        store
            .finish_span(
                &observed.context.run_id,
                &observed.context.span_id,
                terminal,
                "2026-07-23T00:00:03Z",
                Some("tool_error"),
            )
            .unwrap();

        store.start_span(&observed).unwrap();

        let timeline = store
            .timeline(&execution.context.run_id)
            .unwrap()
            .expect("timeline");
        let stored = timeline.spans.first().expect("the span");
        assert_eq!(
            stored.status, terminal,
            "a {terminal:?} span was reopened by a replayed start"
        );
        assert_eq!(stored.ended_at.as_deref(), Some("2026-07-23T00:00:03Z"));
        assert_eq!(stored.error_classification.as_deref(), Some("tool_error"));
    }
}

/// Refusing to regress lifecycle must not also refuse late metadata.
///
/// A producer legitimately learns some correlation identifiers after starting a run. Resolving the
/// conflict with `DO NOTHING` would protect the status and silently drop these, which is why the
/// status alone is conditioned rather than the whole branch.
#[test]
fn a_replayed_start_still_completes_metadata_that_arrives_late() {
    let (_directory, store) = repository("execution-late-start-metadata");
    let execution = run(1, "2026-07-23T00:00:00Z");
    store.start_run(&execution).unwrap();
    store
        .finish_run(
            &execution.context.run_id,
            ExecutionStatus::Succeeded,
            "2026-07-23T00:00:05Z",
            None,
        )
        .unwrap();

    let mut late = execution.clone();
    late.provider_session_id = Some("provider-session-7".to_string());
    late.assistant_message_id = Some("assistant-9".to_string());
    store.start_run(&late).unwrap();

    let timeline = store
        .timeline(&execution.context.run_id)
        .unwrap()
        .expect("timeline");
    assert_eq!(timeline.run.status, ExecutionStatus::Succeeded);
    assert_eq!(
        timeline.run.provider_session_id.as_deref(),
        Some("provider-session-7")
    );
    assert_eq!(
        timeline.run.assistant_message_id.as_deref(),
        Some("assistant-9")
    );
}

/// No ordering may leave a record whose two fields disagree about whether it ended.
///
/// A non-terminal status beside a terminal timestamp forces every reader to guess which one to
/// believe, and readers guess differently.
#[test]
fn no_ordering_leaves_a_running_status_beside_a_terminal_timestamp() {
    for (index, terminal) in TERMINAL_STATUSES.into_iter().enumerate() {
        let (_directory, store) = repository(&format!("execution-order-consistency-{index}"));
        // Offset past zero: an all-zero UUID is not a valid run id.
        let execution = run(index as u64 + 20, "2026-07-23T00:00:00Z");
        let observed = span(&execution);

        store.start_run(&execution).unwrap();
        store.start_span(&observed).unwrap();
        store
            .finish_span(
                &observed.context.run_id,
                &observed.context.span_id,
                terminal,
                "2026-07-23T00:00:02Z",
                None,
            )
            .unwrap();
        store
            .finish_run(
                &execution.context.run_id,
                terminal,
                "2026-07-23T00:00:04Z",
                None,
            )
            .unwrap();
        // Everything replays, in the order a retry would deliver it.
        store.start_run(&execution).unwrap();
        store.start_span(&observed).unwrap();
        store
            .finish_run(
                &execution.context.run_id,
                terminal,
                "2026-07-23T00:00:09Z",
                None,
            )
            .unwrap();

        let timeline = store
            .timeline(&execution.context.run_id)
            .unwrap()
            .expect("timeline");
        assert!(
            timeline.run.ended_at.is_none() || timeline.run.status.is_terminal(),
            "run ended at {:?} while reporting {:?}",
            timeline.run.ended_at,
            timeline.run.status
        );
        for stored in &timeline.spans {
            assert!(
                stored.ended_at.is_none() || stored.status.is_terminal(),
                "span ended at {:?} while reporting {:?}",
                stored.ended_at,
                stored.status
            );
        }
    }
}

/// Records `count` events on one run, the last of which explains a failure.
///
/// The diagnostic goes last on purpose: it is the event a reader is looking for when a run failed,
/// and the position where a silent bound is most damaging.
fn record_events(
    store: &SqliteExecutionTimelineRepository,
    run: &ExecutionRun,
    span: &ExecutionSpan,
    count: u64,
) {
    for sequence in 0..count {
        let last = sequence + 1 == count;
        store
            .record_event(&ExecutionEvent {
                run_id: run.context.run_id.clone(),
                span_id: span.context.span_id.clone(),
                sequence,
                name: if last {
                    "process.failed".to_string()
                } else {
                    "process.output".to_string()
                },
                // Distinct timestamps so the ordering the cursor walks is total and the assertion
                // below is about paging rather than about tie-breaking.
                timestamp: format!("2026-07-23T00:{:02}:{:02}Z", sequence / 60, sequence % 60),
                attributes: SafeAttributes::default(),
            })
            .unwrap();
    }
}

/// A response that stopped at its bound must say so and offer a way past it.
///
/// The bound is injected rather than the production 5000: the paging logic is identical at any
/// bound, and writing 5001 rows would make this test a hundred times slower without covering one
/// additional branch.
#[test]
fn a_truncated_event_page_reports_its_own_truncation() {
    let (_directory, store) = repository("execution-event-truncation");
    let execution = run(7, "2026-07-23T00:00:00Z");
    let observed = span(&execution);
    store.start_run(&execution).unwrap();
    store.start_span(&observed).unwrap();
    record_events(&store, &execution, &observed, 7);

    let first = store
        .timeline_page(&execution.context.run_id, 3, None)
        .unwrap()
        .expect("timeline");

    assert_eq!(first.events.len(), 3);
    assert!(
        first.event_coverage.truncated,
        "a clipped page reported itself as complete"
    );
    assert!(first.event_coverage.next_page_token.is_some());
}

/// A run whose events fit reports complete coverage.
///
/// The counterpart matters as much as the truncation case: a response that always claimed possible
/// truncation would push every reader into a continuation that returns nothing, and they would
/// stop believing the flag.
#[test]
fn an_untruncated_event_page_reports_complete_coverage() {
    let (_directory, store) = repository("execution-event-complete");
    let execution = run(8, "2026-07-23T00:00:00Z");
    let observed = span(&execution);
    store.start_run(&execution).unwrap();
    store.start_span(&observed).unwrap();
    record_events(&store, &execution, &observed, 3);

    let timeline = store
        .timeline_page(&execution.context.run_id, 3, None)
        .unwrap()
        .expect("timeline");

    assert_eq!(timeline.events.len(), 3);
    assert!(!timeline.event_coverage.truncated);
    assert!(timeline.event_coverage.next_page_token.is_none());
}

/// The continuation must actually reach the event that was cut off.
///
/// Reporting truncation without a usable cursor would only tell a reader that something is missing
/// and leave them unable to see it, which is barely better than not telling them.
#[test]
fn the_continuation_reaches_the_diagnostic_event_past_the_bound() {
    let (_directory, store) = repository("execution-event-continuation");
    let execution = run(9, "2026-07-23T00:00:00Z");
    let observed = span(&execution);
    store.start_run(&execution).unwrap();
    store.start_span(&observed).unwrap();
    record_events(&store, &execution, &observed, 7);

    let mut seen = Vec::new();
    let mut token: Option<String> = None;
    // Bounded loop: an off-by-one in the cursor would otherwise spin forever rather than fail.
    for _ in 0..10 {
        let page = store
            .timeline_page(&execution.context.run_id, 3, token.as_deref())
            .unwrap()
            .expect("timeline");
        seen.extend(page.events.iter().map(|event| event.sequence));
        match page.event_coverage.next_page_token {
            Some(next) => token = Some(next),
            None => break,
        }
    }

    // Every event exactly once, in order, including the failure the reader came for.
    assert_eq!(seen, (0..7).collect::<Vec<_>>());
}

/// Paging stays complete even when timestamps are not comparable as times.
///
/// `execution_events.timestamp` is whatever the producer supplied — a provider string when there
/// is one, the clock's `+00:00` RFC 3339 otherwise — so the column mixes `Z` suffixes, varying
/// fractional digits, and non-UTC offsets. Compared as text those do not sort chronologically.
///
/// That misorders the *display*, which `ORDER BY timestamp` has always done and this change does
/// not touch. What it must not do is lose events: the cursor is the previous page's last stored
/// row compared against the same column ordering, so both sides agree on one total order however
/// odd that order looks. A cursor derived from a *parsed* time would not have that property.
#[test]
fn paging_loses_no_events_when_timestamp_formats_are_mixed() {
    let (_directory, store) = repository("execution-event-mixed-formats");
    let execution = run(11, "2026-07-23T00:00:00Z");
    let observed = span(&execution);
    store.start_run(&execution).unwrap();
    store.start_span(&observed).unwrap();

    let stamps = [
        "2026-07-23T10:00:00.100Z",
        "2026-07-23T10:00:00.100000100+00:00",
        "2026-07-23T18:00:00.000+08:00",
        "2026-07-23T10:00:01.000Z",
        "2026-07-23T10:00:02.000000000+00:00",
        "2026-07-23T10:00:03.000Z",
    ];
    for (sequence, timestamp) in stamps.iter().enumerate() {
        store
            .record_event(&ExecutionEvent {
                run_id: execution.context.run_id.clone(),
                span_id: observed.context.span_id.clone(),
                sequence: sequence as u64,
                name: "process.output".to_string(),
                timestamp: (*timestamp).to_string(),
                attributes: SafeAttributes::default(),
            })
            .unwrap();
    }

    let mut seen = Vec::new();
    let mut token: Option<String> = None;
    for _ in 0..10 {
        let page = store
            .timeline_page(&execution.context.run_id, 2, token.as_deref())
            .unwrap()
            .expect("timeline");
        seen.extend(page.events.iter().map(|event| event.sequence));
        match page.event_coverage.next_page_token {
            Some(next) => token = Some(next),
            None => break,
        }
    }

    seen.sort_unstable();
    assert_eq!(
        seen,
        (0..stamps.len() as u64).collect::<Vec<_>>(),
        "paging dropped or repeated an event when timestamp formats were mixed"
    );
}

/// The shipped bound is the one this response has always had.
///
/// Asserted so that "report the truncation" cannot quietly become "return fewer events": the fix
/// is about honesty, and shrinking the page while adding a flag would be a different change.
#[test]
fn the_default_event_bound_is_unchanged() {
    assert_eq!(super::TIMELINE_EVENT_PAGE_SIZE, 5000);
}

/// The paged event read reaches its rows through the index rather than scanning the table.
///
/// Deliberately narrow about what it proves. The plan text names the index and the columns it
/// seeks on, and for this query it reports `(run_id=?)` whichever way the cursor predicate is
/// written — so this cannot distinguish a cursor that seeks from one that filters row by row, and
/// claiming otherwise would make it a guard that passes against the defect it names. What it does
/// catch is the index being dropped or the query drifting off it, which turns every page into a
/// full walk of the run.
#[test]
fn the_paged_event_query_reaches_its_rows_through_the_index() {
    let (_directory, store) = repository("execution-event-plan");
    let connection = store.connection().expect("connection");
    let plan: String = connection
        .query_row(
            "EXPLAIN QUERY PLAN SELECT run_id, span_id, sequence, name, timestamp, attributes_json \
             FROM execution_events WHERE run_id = ?1 \
               AND (?2 IS NULL OR (?2, ?3, ?4) < (timestamp, span_id, sequence)) \
             ORDER BY timestamp, span_id, sequence LIMIT ?5",
            params!["run-1", "2026-07-23T00:00:00Z", "span-1", 1_i64, 10_i64],
            |row| row.get(3),
        )
        .expect("query plan");
    assert!(
        plan.contains("idx_execution_events_run_time"),
        "expected the run/time index, got: {plan}"
    );
    assert!(
        !plan.contains("SCAN execution_events"),
        "the paged event read degraded to a scan: {plan}"
    );
}
