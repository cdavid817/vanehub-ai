use super::models::{
    EvidenceNotice, EvidenceNoticeKind, EvidenceQueryScope, ExecutionRecordFilters,
    ExecutionRecordKind, UnownedSummarySource, DEFAULT_EVIDENCE_PAGE_SIZE, MAX_EVIDENCE_PAGE_SIZE,
};
use super::ports::EvidenceAppendOutcome;
use super::service::{bounded_page_size, capture_not_initialized_coverage};
use super::*;
use crate::contexts::execution_observability::domain::evidence::builders::{
    reason, CorrelationBuilder, CoverageBuilder,
};
use crate::contexts::execution_observability::domain::evidence::payload::EvidenceOutcome;
use crate::contexts::execution_observability::domain::{
    reason_codes, EvidenceCoverageState, EvidenceSessionId, EvidenceSourceContext,
    ExecutionEvidenceEvent, ExecutionFidelity, ExecutionStatus, QueryCoverage, RedactionReceipt,
    SafeEvidencePayload, SafeReasonCode, SourceEventId,
};
use std::sync::{Arc, Mutex};

const SESSION: &str = "session-1";
const RUN: &str = "6f1b2c3d-4e5f-4a6b-8c9d-0e1f2a3b4c5d";
const TRACE: &str = "0af7651916cd43dd8448eb211c80319c";

/// Fake ports. None of them touch SQLite, Tauri, the filesystem, the network, or a process, which
/// is the point: the use cases own the ordering rules, and those rules must be provable without
/// standing up any of the machinery they will eventually run against.
#[derive(Default)]
struct FakeRepository {
    outcome: Mutex<Option<EvidenceAppendOutcome>>,
    appended: Mutex<Vec<String>>,
    listed_limits: Mutex<Vec<usize>>,
    fail_append: Mutex<bool>,
    projection_stale: Mutex<bool>,
    fail_replay: Mutex<bool>,
    replay_calls: Mutex<usize>,
}

impl FakeRepository {
    fn with_outcome(outcome: EvidenceAppendOutcome) -> Arc<Self> {
        let repository = Self::default();
        *repository.outcome.lock().expect("outcome") = Some(outcome);
        Arc::new(repository)
    }
}

impl EvidenceRepositoryPort for FakeRepository {
    fn append(
        &self,
        event: &ExecutionEvidenceEvent,
        fingerprint: &str,
        _recorded_at: &str,
    ) -> Result<EvidenceAppendOutcome, EvidenceApplicationError> {
        if *self.fail_append.lock().expect("fail flag") {
            return Err(EvidenceApplicationError::Storage("disk".to_string()));
        }
        self.appended.lock().expect("appended").push(format!(
            "{}:{fingerprint}",
            event.source_event_id().as_str()
        ));
        Ok(self
            .outcome
            .lock()
            .expect("outcome")
            .clone()
            .unwrap_or(EvidenceAppendOutcome::Appended { sequence: 1 }))
    }

    fn list_records(
        &self,
        query: &ExecutionRecordQuery,
    ) -> Result<EvidenceRecordPage, EvidenceApplicationError> {
        self.listed_limits.lock().expect("limits").push(query.limit);
        Ok(EvidenceRecordPage {
            items: Vec::new(),
            next_cursor: None,
            coverage: CoverageBuilder::capture_not_initialized(),
        })
    }

    fn record_detail(
        &self,
        _query: &ExecutionRecordDetailQuery,
    ) -> Result<ExecutionRecordDetailView, EvidenceApplicationError> {
        Err(EvidenceApplicationError::RecordNotFound)
    }

    fn summary(
        &self,
        query: &WorkspaceEvidenceSummaryQuery,
    ) -> Result<WorkspaceEvidenceSummary, EvidenceApplicationError> {
        Ok(WorkspaceEvidenceSummary {
            session_id: query.session_id.clone(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            coverage: capture_not_initialized_coverage(),
            run_status: None,
            run_id: None,
            run_started_at: None,
            running_records: 0,
            failed_records: 0,
            verification_passed: 0,
            verification_failed: 0,
            unowned_sources: vec![UnownedSummarySource {
                source: "logs",
                coverage_state: EvidenceCoverageState::Unavailable,
                reason_code: reason_codes::SOURCE_NOT_OWNED,
            }],
        })
    }

    fn correlation_counts(
        &self,
        _session_id: &EvidenceSessionId,
        _run_id: Option<&str>,
    ) -> Result<EvidenceCorrelationCounts, EvidenceApplicationError> {
        Ok(EvidenceCorrelationCounts::default())
    }

    fn subscription_bootstrap(
        &self,
        session_id: &EvidenceSessionId,
    ) -> Result<EvidenceSubscriptionBootstrap, EvidenceApplicationError> {
        Ok(EvidenceSubscriptionBootstrap {
            session_id: session_id.clone(),
            watermark_sequence: 7,
            coverage: QueryCoverage::complete(),
        })
    }

    fn projection_is_stale(&self) -> Result<bool, EvidenceApplicationError> {
        Ok(*self.projection_stale.lock().expect("stale flag"))
    }

    fn replay_projections(
        &self,
        _session_id: Option<&EvidenceSessionId>,
    ) -> Result<usize, EvidenceApplicationError> {
        *self.replay_calls.lock().expect("replay calls") += 1;
        if *self.fail_replay.lock().expect("fail flag") {
            return Err(EvidenceApplicationError::Storage("disk".to_string()));
        }
        Ok(7)
    }

    fn maintain_retention(
        &self,
        _cutoff: &str,
        _now: &str,
    ) -> Result<EvidenceRetentionSummary, EvidenceApplicationError> {
        Ok(EvidenceRetentionSummary::default())
    }
}

struct FixedClock;

impl EvidenceClockPort for FixedClock {
    fn now_rfc3339(&self) -> String {
        "2026-01-01T00:00:00.000Z".to_string()
    }
}

#[derive(Default)]
struct SequentialIds {
    next: Mutex<u32>,
}

impl EvidenceIdGeneratorPort for SequentialIds {
    fn next_event_id(&self) -> String {
        let mut next = self.next.lock().expect("id counter");
        *next += 1;
        format!("event-{next}")
    }
}

#[derive(Default)]
struct RecordingValidator {
    reject: Mutex<bool>,
    seen: Mutex<usize>,
}

impl EvidenceRedactionValidatorPort for RecordingValidator {
    fn validate(&self, _event: &ExecutionEvidenceEvent) -> Result<(), EvidenceApplicationError> {
        *self.seen.lock().expect("seen") += 1;
        if *self.reject.lock().expect("reject") {
            return Err(EvidenceApplicationError::Storage("policy".to_string()));
        }
        Ok(())
    }
}

#[derive(Default)]
struct CapturingNotices {
    published: Mutex<Vec<EvidenceNotice>>,
}

impl PostCommitEvidenceNoticePublisherPort for CapturingNotices {
    fn publish(&self, notice: &EvidenceNotice) {
        self.published.lock().expect("notices").push(notice.clone());
    }
}

#[derive(Default)]
struct CapturingDiagnostics {
    conflicts: Mutex<Vec<String>>,
    drops: Mutex<Vec<u32>>,
}

impl EvidenceGapDiagnosticsPort for CapturingDiagnostics {
    fn record_conflict(
        &self,
        source_context: EvidenceSourceContext,
        source_event_id: &SourceEventId,
    ) {
        self.conflicts.lock().expect("conflicts").push(format!(
            "{}:{}",
            source_context.as_str(),
            source_event_id.as_str()
        ));
    }

    fn record_dropped(&self, _session_id: &EvidenceSessionId, dropped_count: u32) {
        self.drops.lock().expect("drops").push(dropped_count);
    }
}

struct Harness {
    service: ExecutionEvidenceService,
    repository: Arc<FakeRepository>,
    validator: Arc<RecordingValidator>,
    notices: Arc<CapturingNotices>,
    diagnostics: Arc<CapturingDiagnostics>,
}

fn harness(outcome: EvidenceAppendOutcome) -> Harness {
    let repository = FakeRepository::with_outcome(outcome);
    let validator = Arc::new(RecordingValidator::default());
    let notices = Arc::new(CapturingNotices::default());
    let diagnostics = Arc::new(CapturingDiagnostics::default());
    Harness {
        service: ExecutionEvidenceService::new(
            repository.clone(),
            Arc::new(FixedClock),
            Arc::new(SequentialIds::default()),
            validator.clone(),
            notices.clone(),
            diagnostics.clone(),
        ),
        repository,
        validator,
        notices,
        diagnostics,
    }
}

fn run_completed_input(source_event_id: &str) -> RecordEvidenceInput {
    RecordEvidenceInput {
        source_context: EvidenceSourceContext::AgentRuntime,
        source_event_id: SourceEventId::parse(source_event_id).expect("source id"),
        occurred_at: "2026-01-01T00:00:00.000Z".to_string(),
        correlation: CorrelationBuilder::for_session(SESSION)
            .with_run(RUN, TRACE)
            .build(),
        status: Some(ExecutionStatus::Succeeded),
        fidelity: ExecutionFidelity::Native,
        payload: SafeEvidencePayload::RunCompleted {
            outcome: EvidenceOutcome::Succeeded,
            duration_ms: Some(1_200),
        },
        redaction: RedactionReceipt::none(),
    }
}

#[test]
fn recording_validates_persists_then_publishes_exactly_once() {
    let harness = harness(EvidenceAppendOutcome::Appended { sequence: 12 });

    let outcome = harness
        .service
        .record(run_completed_input("source-1"))
        .expect("recorded");

    assert_eq!(outcome, RecordEvidenceOutcome::Recorded { sequence: 12 });
    assert_eq!(*harness.validator.seen.lock().expect("seen"), 1);
    assert_eq!(
        harness.repository.appended.lock().expect("appended").len(),
        1
    );
    let published = harness.notices.published.lock().expect("notices");
    assert_eq!(published.len(), 1);
    assert_eq!(published[0].sequence, 12);
    assert_eq!(published[0].kind, EvidenceNoticeKind::SummaryChanged);
}

// A retry did the right thing; publishing again would make a subscriber count the record twice.
#[test]
fn an_identical_duplicate_succeeds_without_publishing_again() {
    let harness = harness(EvidenceAppendOutcome::IdenticalDuplicate { sequence: 3 });

    let outcome = harness
        .service
        .record(run_completed_input("source-1"))
        .expect("idempotent success");

    assert_eq!(outcome, RecordEvidenceOutcome::Duplicate { sequence: 3 });
    assert!(harness
        .notices
        .published
        .lock()
        .expect("notices")
        .is_empty());
    assert!(harness
        .diagnostics
        .conflicts
        .lock()
        .expect("conflicts")
        .is_empty());
}

#[test]
fn a_conflicting_duplicate_reports_a_diagnostic_and_publishes_nothing() {
    let harness = harness(EvidenceAppendOutcome::Conflict);

    let outcome = harness
        .service
        .record(run_completed_input("source-1"))
        .expect("conflict is reported, not thrown");

    assert_eq!(outcome, RecordEvidenceOutcome::Conflict);
    assert!(harness
        .notices
        .published
        .lock()
        .expect("notices")
        .is_empty());
    assert_eq!(
        *harness.diagnostics.conflicts.lock().expect("conflicts"),
        vec!["agent_runtime:source-1".to_string()]
    );
}

#[test]
fn an_invalid_event_never_reaches_storage() {
    let harness = harness(EvidenceAppendOutcome::Appended { sequence: 1 });
    let mut input = run_completed_input("source-1");
    // A run completion without its run id: the projection could never join it to anything.
    input.correlation = CorrelationBuilder::for_session(SESSION).build();

    let error = harness.service.record(input).expect_err("refused");

    assert!(matches!(error, EvidenceApplicationError::Domain(_)));
    assert!(harness
        .repository
        .appended
        .lock()
        .expect("appended")
        .is_empty());
    assert!(harness
        .notices
        .published
        .lock()
        .expect("notices")
        .is_empty());
}

#[test]
fn a_rejected_redaction_policy_stops_the_write() {
    let harness = harness(EvidenceAppendOutcome::Appended { sequence: 1 });
    *harness.validator.reject.lock().expect("reject") = true;

    let error = harness
        .service
        .record(run_completed_input("source-1"))
        .expect_err("policy rejection");

    assert!(matches!(error, EvidenceApplicationError::Storage(_)));
    assert!(harness
        .repository
        .appended
        .lock()
        .expect("appended")
        .is_empty());
}

#[test]
fn a_storage_failure_publishes_no_notice() {
    let harness = harness(EvidenceAppendOutcome::Appended { sequence: 1 });
    *harness.repository.fail_append.lock().expect("fail flag") = true;

    let error = harness
        .service
        .record(run_completed_input("source-1"))
        .expect_err("storage failure");

    assert!(matches!(error, EvidenceApplicationError::Storage(_)));
    assert!(harness
        .notices
        .published
        .lock()
        .expect("notices")
        .is_empty());
}

#[test]
fn a_command_event_publishes_a_record_notice_carrying_only_identifiers() {
    let harness = harness(EvidenceAppendOutcome::Appended { sequence: 5 });
    let input = RecordEvidenceInput {
        source_context: EvidenceSourceContext::Workspaces,
        source_event_id: SourceEventId::parse("source-command").expect("source id"),
        occurred_at: "2026-01-01T00:00:00.000Z".to_string(),
        correlation: CorrelationBuilder::for_session(SESSION)
            .with_run(RUN, TRACE)
            .with_command("command-1")
            .with_seat("seat-builder")
            .build(),
        status: Some(ExecutionStatus::Failed),
        fidelity: ExecutionFidelity::Native,
        payload: SafeEvidencePayload::CommandCompleted {
            outcome: EvidenceOutcome::Failed,
            duration_ms: Some(12_400),
            exit_code: Some(1),
            signal: None,
            output_availability:
                crate::contexts::execution_observability::domain::OutputAvailability::Merged,
            output_truncated: true,
        },
        redaction: RedactionReceipt::none(),
    };

    harness.service.record(input).expect("recorded");

    let published = harness.notices.published.lock().expect("notices");
    let notice = &published[0];
    // A completion lands on the record its start created, so the notice says the row changed
    // rather than that a new one appeared. A subscriber told "appended" would insert a duplicate.
    assert_eq!(notice.kind, EvidenceNoticeKind::RecordUpdated);
    assert_eq!(notice.command_id.as_deref(), Some("command-1"));
    assert_eq!(notice.run_id.as_deref(), Some(RUN));
    assert!(notice.dropped_count.is_none());
    // Nothing derived from the payload appears on the event channel.
    let rendered = format!("{notice:?}");
    assert!(!rendered.contains("12400"));
    assert!(!rendered.contains("exit"));
}

#[test]
fn page_size_is_bounded_before_the_query_runs() {
    let harness = harness(EvidenceAppendOutcome::Appended { sequence: 1 });
    let query = |limit| ExecutionRecordQuery {
        scope: EvidenceQueryScope {
            session_id: Some(EvidenceSessionId::parse(SESSION).expect("session")),
            ..EvidenceQueryScope::default()
        },
        filters: ExecutionRecordFilters::default(),
        cursor: None,
        limit,
    };

    harness
        .service
        .list_records(query(0))
        .expect("default page");
    harness
        .service
        .list_records(query(9_000))
        .expect("clamped page");
    harness
        .service
        .list_records(query(25))
        .expect("explicit page");

    assert_eq!(
        *harness.repository.listed_limits.lock().expect("limits"),
        vec![DEFAULT_EVIDENCE_PAGE_SIZE, MAX_EVIDENCE_PAGE_SIZE, 25]
    );
}

#[test]
fn bounded_page_size_never_returns_zero_or_more_than_the_maximum() {
    assert_eq!(bounded_page_size(0), DEFAULT_EVIDENCE_PAGE_SIZE);
    assert_eq!(bounded_page_size(1), 1);
    assert_eq!(
        bounded_page_size(MAX_EVIDENCE_PAGE_SIZE + 1),
        MAX_EVIDENCE_PAGE_SIZE
    );
}

// Before Task Group 4 there is no producer, so an empty answer says nothing about whether work
// happened. Reporting `complete` here is the one thing this capability must never do.
#[test]
fn an_unwired_store_reports_partial_capture_rather_than_a_confident_zero() {
    let coverage = capture_not_initialized_coverage();
    assert_eq!(coverage.state(), EvidenceCoverageState::Partial);
    assert_eq!(
        coverage
            .reason_codes()
            .iter()
            .map(SafeReasonCode::as_str)
            .collect::<Vec<_>>(),
        vec![reason_codes::CAPTURE_NOT_INITIALIZED]
    );
}

#[test]
fn a_summary_marks_sources_this_context_does_not_own() {
    let harness = harness(EvidenceAppendOutcome::Appended { sequence: 1 });
    let summary = harness
        .service
        .summary(WorkspaceEvidenceSummaryQuery {
            session_id: EvidenceSessionId::parse(SESSION).expect("session"),
            seat_id: None,
        })
        .expect("summary");

    assert_eq!(summary.coverage.state(), EvidenceCoverageState::Partial);
    assert_eq!(summary.unowned_sources.len(), 1);
    assert_eq!(summary.unowned_sources[0].source, "logs");
    assert_eq!(
        summary.unowned_sources[0].coverage_state,
        EvidenceCoverageState::Unavailable
    );
}

#[test]
fn a_bootstrap_returns_the_committed_watermark() {
    let harness = harness(EvidenceAppendOutcome::Appended { sequence: 1 });
    let bootstrap = harness
        .service
        .subscription_bootstrap(&EvidenceSessionId::parse(SESSION).expect("session"))
        .expect("bootstrap");
    assert_eq!(bootstrap.watermark_sequence, 7);
}

#[test]
fn a_reported_drop_emits_one_gap_notice_and_ignores_an_empty_one() {
    let harness = harness(EvidenceAppendOutcome::Appended { sequence: 1 });
    let session = EvidenceSessionId::parse(SESSION).expect("session");

    harness.service.record_dropped_events(&session, 0);
    assert!(harness
        .notices
        .published
        .lock()
        .expect("notices")
        .is_empty());

    harness.service.record_dropped_events(&session, 4);
    let published = harness.notices.published.lock().expect("notices");
    assert_eq!(published.len(), 1);
    assert_eq!(published[0].kind, EvidenceNoticeKind::CoverageGap);
    assert_eq!(published[0].dropped_count, Some(4));
    assert_eq!(*harness.diagnostics.drops.lock().expect("drops"), vec![4]);
}

#[test]
fn record_kinds_map_only_the_lifecycles_that_project_into_records() {
    use crate::contexts::execution_observability::domain::EvidenceKind;
    assert_eq!(
        ExecutionRecordKind::for_kind(EvidenceKind::CommandStarted),
        Some(ExecutionRecordKind::Command)
    );
    assert_eq!(
        ExecutionRecordKind::for_kind(EvidenceKind::ToolCompleted),
        Some(ExecutionRecordKind::Tool)
    );
    assert_eq!(
        ExecutionRecordKind::for_kind(EvidenceKind::AgentDelegated),
        Some(ExecutionRecordKind::Delegation)
    );
    assert_eq!(
        ExecutionRecordKind::for_kind(EvidenceKind::VerificationCompleted),
        Some(ExecutionRecordKind::Verification)
    );
    // Shell, file, review, usage, and gap events are evidence but not execution records.
    assert_eq!(
        ExecutionRecordKind::for_kind(EvidenceKind::ShellOpened),
        None
    );
    assert_eq!(
        ExecutionRecordKind::for_kind(EvidenceKind::CoverageGapRecorded),
        None
    );
}

#[test]
fn record_kind_tokens_round_trip() {
    for kind in [
        ExecutionRecordKind::Command,
        ExecutionRecordKind::Tool,
        ExecutionRecordKind::Delegation,
        ExecutionRecordKind::Verification,
    ] {
        assert_eq!(ExecutionRecordKind::parse(kind.as_str()), Some(kind));
    }
    assert_eq!(ExecutionRecordKind::parse("legacy"), None);
    let _ = reason("unused_helper_guard");
}

/// A projection that already agrees with the journal must not be rebuilt.
///
/// The cost of getting this wrong is invisible in a test suite and obvious on a large store: every
/// launch would read every event ever recorded, to produce rows byte-identical to the ones it just
/// deleted. Asserting on the replay call count rather than on the result is what catches it, since
/// both branches return successfully.
#[test]
fn a_current_projection_is_not_rebuilt_at_startup() {
    let harness = harness(EvidenceAppendOutcome::Appended { sequence: 1 });
    *harness.repository.projection_stale.lock().expect("flag") = false;

    let outcome = harness
        .service
        .repair_projections_if_needed()
        .expect("repair");

    assert_eq!(outcome, ProjectionRepair::AlreadyCurrent);
    assert_eq!(*harness.repository.replay_calls.lock().expect("calls"), 0);
}

#[test]
fn a_lagging_projection_is_rebuilt_at_startup() {
    let harness = harness(EvidenceAppendOutcome::Appended { sequence: 1 });
    *harness.repository.projection_stale.lock().expect("flag") = true;

    let outcome = harness
        .service
        .repair_projections_if_needed()
        .expect("repair");

    assert_eq!(outcome, ProjectionRepair::Rebuilt { records: 7 });
    assert_eq!(*harness.repository.replay_calls.lock().expect("calls"), 1);
}

/// A failed rebuild leaves the stale projection in place and says so by failing. What it must not
/// do is report success: the coverage rules read the projection's own lag to choose between
/// `indexing` and `complete`, so a swallowed failure would leave a lagging projection answering as
/// though it were whole.
#[test]
fn a_failed_rebuild_reports_the_failure_rather_than_claiming_success() {
    let harness = harness(EvidenceAppendOutcome::Appended { sequence: 1 });
    *harness.repository.projection_stale.lock().expect("flag") = true;
    *harness.repository.fail_replay.lock().expect("flag") = true;

    let error = harness
        .service
        .repair_projections_if_needed()
        .expect_err("storage failure");

    assert!(matches!(error, EvidenceApplicationError::Storage(_)));
}

/// Startup repair reads and rewrites the projection. It must never append to the journal: an event
/// minted at startup is, once persisted, indistinguishable from an observation of real work.
#[test]
fn startup_repair_appends_no_evidence_and_publishes_no_notice() {
    let harness = harness(EvidenceAppendOutcome::Appended { sequence: 1 });
    *harness.repository.projection_stale.lock().expect("flag") = true;

    harness
        .service
        .repair_projections_if_needed()
        .expect("repair");

    assert!(harness
        .repository
        .appended
        .lock()
        .expect("appended")
        .is_empty());
    assert!(harness
        .notices
        .published
        .lock()
        .expect("notices")
        .is_empty());
    assert_eq!(*harness.validator.seen.lock().expect("validated"), 0);
}

/// A start and a completion describe the same record, and a subscriber has to be able to tell
/// which one it is holding: the first creates a row, the second changes one that is already on
/// screen. Collapsing both into `record-appended` is what produces duplicate rows in a live list.
#[test]
fn a_start_appends_and_a_completion_updates() {
    let harness = harness(EvidenceAppendOutcome::Appended { sequence: 1 });
    let correlation = || {
        CorrelationBuilder::for_session(SESSION)
            .with_run(RUN, TRACE)
            .with_command("command-1")
            .build()
    };

    harness
        .service
        .record(RecordEvidenceInput {
            source_context: EvidenceSourceContext::Workspaces,
            source_event_id: SourceEventId::parse("start").expect("source id"),
            occurred_at: "2026-01-01T00:00:00.000Z".to_string(),
            correlation: correlation(),
            status: Some(ExecutionStatus::Running),
            fidelity: ExecutionFidelity::Native,
            payload: SafeEvidencePayload::CommandStarted {
                runtime_kind: crate::contexts::execution_observability::domain::CommandRuntimeKind::LocalShell,
                redacted_display: None,
                cwd_display: None,
            },
            redaction: RedactionReceipt::none(),
        })
        .expect("start");
    harness
        .service
        .record(RecordEvidenceInput {
            source_context: EvidenceSourceContext::Workspaces,
            source_event_id: SourceEventId::parse("done").expect("source id"),
            occurred_at: "2026-01-01T00:00:05.000Z".to_string(),
            correlation: correlation(),
            status: Some(ExecutionStatus::Succeeded),
            fidelity: ExecutionFidelity::Native,
            payload: SafeEvidencePayload::CommandCompleted {
                outcome: EvidenceOutcome::Succeeded,
                duration_ms: None,
                exit_code: Some(0),
                signal: None,
                output_availability:
                    crate::contexts::execution_observability::domain::OutputAvailability::Merged,
                output_truncated: false,
            },
            redaction: RedactionReceipt::none(),
        })
        .expect("completion");

    let published = harness.notices.published.lock().expect("notices");
    assert_eq!(published[0].kind, EvidenceNoticeKind::RecordAppended);
    assert_eq!(published[1].kind, EvidenceNoticeKind::RecordUpdated);
}

/// A non-lifecycle event changes counts without producing a row, so it is neither an append nor an
/// update. Reporting it as either would make a subscriber look for a record that does not exist.
#[test]
fn a_non_lifecycle_event_reports_a_summary_change() {
    let harness = harness(EvidenceAppendOutcome::Appended { sequence: 1 });

    harness
        .service
        .record(run_completed_input("source-run"))
        .expect("recorded");

    let published = harness.notices.published.lock().expect("notices");
    assert_eq!(published[0].kind, EvidenceNoticeKind::SummaryChanged);
}
