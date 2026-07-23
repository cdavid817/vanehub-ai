use super::SqliteExecutionTimelineRepository;
use crate::contexts::execution_observability::application::ExecutionTelemetryPort;
use crate::contexts::execution_observability::domain::{
    CapturePolicy, ExecutionContext, ExecutionEvent, ExecutionFidelity, ExecutionRun,
    ExecutionRunId, ExecutionSource, ExecutionSpan, ExecutionStatus, ObservabilitySettings,
    PageRequest, SafeAttributes, SpanId, TraceId,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

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
