//! What a report is willing to claim, and what it refuses to turn into a number.
//!
//! Most of these are about a source that could not answer. That is the ordinary case rather than
//! the exceptional one — a report reads from five contexts, and the chance that all five are
//! healthy and caught up is not high on the day somebody opens a report. The failure to prevent is
//! specific: a missing measurement rendered as zero, in a document somebody quotes.

use super::models::{
    AgentReportRow, CommandReport, FailureReportRow, ReportCoverageState, ReportGroupBy,
    ToolReportRow, VerificationReport,
};
use super::ports::*;
use super::scope::{ReportScope, ReportScopeError, ReportScopeRequest};
use super::service::{SessionRunReportService, MAX_FAILURE_ROWS};
use std::sync::Arc;

const SESSION: &str = "session-1";

#[derive(Default)]
struct Sources {
    runs: Option<RunOutcomeSummary>,
    evidence: Option<ExecutionEvidenceSummary>,
    timings: Option<TimingSummary>,
    logs: Option<LogFailureSummary>,
    changes: Option<ChangeSummary>,
    usage: Option<ReportUsageSummary>,
}

/// Each port answers with what it was given, or refuses. `None` is a refusal, which is how a test
/// stages the case that matters most.
struct Stub(Sources);

impl RunOutcomePort for Stub {
    fn run_outcomes(&self, _scope: &ReportScope) -> ReportSourceResult<RunOutcomeSummary> {
        self.0
            .runs
            .clone()
            .ok_or(ReportSourceError::Unavailable("runs_unavailable"))
    }
}

impl ExecutionEvidencePort for Stub {
    fn execution_evidence(
        &self,
        _scope: &ReportScope,
    ) -> ReportSourceResult<ExecutionEvidenceSummary> {
        self.0
            .evidence
            .clone()
            .ok_or(ReportSourceError::Unavailable("evidence_unavailable"))
    }
}

impl ObservabilityTimingPort for Stub {
    fn timings(&self, _scope: &ReportScope) -> ReportSourceResult<TimingSummary> {
        self.0
            .timings
            .clone()
            .ok_or(ReportSourceError::Unavailable("timings_unavailable"))
    }
}

impl LogFailurePort for Stub {
    fn log_failures(&self, _scope: &ReportScope) -> ReportSourceResult<LogFailureSummary> {
        self.0
            .logs
            .clone()
            .ok_or(ReportSourceError::Unavailable("logs_unavailable"))
    }
}

impl ChangeSummaryPort for Stub {
    fn changes(&self, _scope: &ReportScope) -> ReportSourceResult<ChangeSummary> {
        self.0
            .changes
            .clone()
            .ok_or(ReportSourceError::Unavailable("changes_unavailable"))
    }
}

impl ReportUsagePort for Stub {
    fn usage(&self, _scope: &ReportScope) -> ReportSourceResult<ReportUsageSummary> {
        self.0
            .usage
            .clone()
            .ok_or(ReportSourceError::Unavailable("usage_unavailable"))
    }
}

struct FixedClock;

impl ReportClock for FixedClock {
    fn now(&self) -> String {
        "2026-08-25T10:00:00Z".to_string()
    }
}

fn service(sources: Sources) -> SessionRunReportService {
    let stub = Arc::new(Stub(sources));
    SessionRunReportService::new(
        stub.clone(),
        stub.clone(),
        stub.clone(),
        stub.clone(),
        stub.clone(),
        stub,
        Arc::new(FixedClock),
    )
}

/// Every source healthy, so every section can be complete.
fn healthy() -> Sources {
    Sources {
        runs: Some(RunOutcomeSummary {
            run_count: 3,
            succeeded: 2,
            failed: 1,
            cancelled: 0,
            retries: 1,
            total_duration_ms: Some(9_000),
            agents: vec![AgentReportRow {
                agent_id: Some("claude-code".to_string()),
                run_count: 3,
                failed_count: 1,
                duration_ms: Some(9_000),
                ..AgentReportRow::default()
            }],
            incomplete: false,
        }),
        evidence: Some(ExecutionEvidenceSummary {
            tools: vec![ToolReportRow {
                tool_name: "read_file".to_string(),
                invocations: 12,
                failures: 1,
                duration_ms: Some(1_200),
            }],
            commands: CommandReport {
                total: 4,
                failed: 1,
                running: 0,
                duration_ms: Some(3_000),
            },
            verification: VerificationReport {
                passed: 10,
                failed: 2,
                skipped: 1,
            },
            failures: vec![FailureReportRow {
                reason_code: "tool_timeout".to_string(),
                count: 1,
                target: None,
            }],
            incomplete: false,
        }),
        timings: Some(TimingSummary {
            p50_ms: Some(400),
            p95_ms: Some(2_100),
            slowest_record_duration_ms: Some(5_000),
            incomplete: false,
        }),
        logs: Some(LogFailureSummary::default()),
        changes: Some(ChangeSummary {
            changed_files: 7,
            unviewed_files: 2,
            unresolved_findings: 1,
            incomplete: false,
        }),
        usage: Some(ReportUsageSummary {
            reported_input_tokens: Some(1_000),
            reported_output_tokens: Some(500),
            reported_derived_tokens: Some(120),
            estimated_characters: Some(40_000),
            response_count: 9,
            internal_purpose_response_count: 3,
            incomplete: false,
        }),
    }
}

fn request() -> ReportScopeRequest {
    ReportScopeRequest {
        session_id: SESSION.to_string(),
        ..ReportScopeRequest::default()
    }
}

// ---------------------------------------------------------------------------------------------
// 10.6 — coverage rather than substituted zeroes
// ---------------------------------------------------------------------------------------------

#[test]
fn a_report_with_every_source_healthy_is_complete() {
    let report = service(healthy()).report(request()).expect("report");

    assert_eq!(report.coverage.overall(), ReportCoverageState::Complete);
    assert_eq!(report.overview.run_count, 3);
    assert_eq!(report.changes.changed_files, 7);
}

/// One unavailable source degrades one section, not the report.
///
/// A report that refused to answer because its change summary was missing would be useless in
/// exactly the situation somebody opens one.
#[test]
fn an_unavailable_source_degrades_only_its_own_section() {
    let mut sources = healthy();
    sources.changes = None;

    let report = service(sources).report(request()).expect("report");

    assert_eq!(
        report.coverage.sections["changes"].state,
        ReportCoverageState::Unavailable
    );
    assert_eq!(
        report.coverage.sections["overview"].state,
        ReportCoverageState::Complete
    );
    assert_eq!(
        report.overview.run_count, 3,
        "a healthy section still answered"
    );
}

/// The zero beside an unavailable section is not a measurement, and the coverage is what says so.
#[test]
fn an_unavailable_section_reports_no_measured_figure() {
    let mut sources = healthy();
    sources.timings = None;

    let report = service(sources).report(request()).expect("report");

    // Absent, not zero. A p50 of zero would report a session where every call returned instantly,
    // which is a specific and false claim rather than a missing one.
    assert_eq!(report.latency.p50_ms, None);
    assert_eq!(report.latency.p95_ms, None);
    assert_eq!(
        report.coverage.sections["latency"].reason_codes,
        vec!["timings_unavailable"]
    );
}

/// A source that answered but knows it is short makes its section partial.
#[test]
fn a_short_answer_is_partial_rather_than_complete() {
    let mut sources = healthy();
    sources.evidence = Some(ExecutionEvidenceSummary {
        incomplete: true,
        ..sources.evidence.take().expect("evidence")
    });

    let report = service(sources).report(request()).expect("report");

    for section in ["tools", "commands", "verification"] {
        assert_eq!(
            report.coverage.sections[section].state,
            ReportCoverageState::Partial,
            "{section} claimed to be complete"
        );
    }
}

/// The report's own state is the weakest of its sections.
#[test]
fn the_overall_state_is_the_weakest_section() {
    let mut sources = healthy();
    sources.usage = None;

    let report = service(sources).report(request()).expect("report");

    // A report whose overall coverage read `complete` while a section was unavailable would be
    // asserting exactly what that section declined to.
    assert_eq!(report.coverage.overall(), ReportCoverageState::Unavailable);
}

/// A report with no evidence at all is unavailable everywhere, and says so nine times.
#[test]
fn a_report_with_no_sources_is_unavailable_in_every_section() {
    let report = service(Sources::default())
        .report(request())
        .expect("report");

    assert_eq!(report.coverage.overall(), ReportCoverageState::Unavailable);
    for section in super::models::REPORT_SECTIONS {
        assert_eq!(
            report.coverage.sections[section].state,
            ReportCoverageState::Unavailable,
            "{section} did not report its own unavailability"
        );
    }
}

/// A session that genuinely did nothing is complete with zeroes, and that is a different fact.
#[test]
fn a_session_that_did_nothing_reports_complete_zeroes() {
    let sources = Sources {
        runs: Some(RunOutcomeSummary::default()),
        evidence: Some(ExecutionEvidenceSummary::default()),
        timings: Some(TimingSummary::default()),
        logs: Some(LogFailureSummary::default()),
        changes: Some(ChangeSummary::default()),
        usage: Some(ReportUsageSummary::default()),
    };

    let report = service(sources).report(request()).expect("report");

    // The zeroes are the same zeroes an unavailable report carries. Only the coverage tells a
    // reader which of the two they are looking at, which is the whole reason it exists.
    assert_eq!(report.coverage.overall(), ReportCoverageState::Complete);
    assert_eq!(report.overview.run_count, 0);
}

// ---------------------------------------------------------------------------------------------
// 10.5 — usage qualities stay apart
// ---------------------------------------------------------------------------------------------

#[test]
fn reported_derived_and_estimated_usage_are_carried_separately() {
    let report = service(healthy()).report(request()).expect("report");

    assert_eq!(report.usage.reported_input_tokens, Some(1_000));
    assert_eq!(report.usage.reported_output_tokens, Some(500));
    assert_eq!(report.usage.reported_derived_tokens, Some(120));
    // Characters, not tokens. Naming an estimate in the same unit as a measurement is what invites
    // somebody to add them.
    assert_eq!(report.usage.estimated_characters, Some(40_000));
}

#[test]
fn internal_purpose_consumption_stays_separate_from_the_user_facing_count() {
    let report = service(healthy()).report(request()).expect("report");

    // "What did this session cost me" and "what did it show me" are different questions, and one
    // number cannot answer both.
    assert_eq!(report.usage.response_count, 9);
    assert_eq!(report.usage.internal_purpose_response_count, 3);
}

#[test]
fn a_session_with_no_usage_reports_absent_figures_rather_than_zero_tokens() {
    let mut sources = healthy();
    sources.usage = Some(ReportUsageSummary {
        response_count: 0,
        ..ReportUsageSummary::default()
    });

    let report = service(sources).report(request()).expect("report");

    // Nobody reported a token count, so there is none. Zero would say the model was called and
    // consumed nothing.
    assert_eq!(report.usage.reported_input_tokens, None);
    assert_eq!(report.usage.response_count, 0);
}

// ---------------------------------------------------------------------------------------------
// 10.7 — no monetary cost
// ---------------------------------------------------------------------------------------------

#[test]
fn a_report_never_claims_a_monetary_cost() {
    let healthy_report = service(healthy()).report(request()).expect("report");
    let empty_report = service(Sources::default())
        .report(request())
        .expect("report");

    // No pricing catalog exists in this change, so a cost would be computed from a rate nobody
    // could check later against what was actually charged.
    assert!(!healthy_report.usage.cost_available);
    assert!(!empty_report.usage.cost_available);
}

// ---------------------------------------------------------------------------------------------
// 10.4 — bounded scope
// ---------------------------------------------------------------------------------------------

#[test]
fn a_report_without_a_session_is_refused() {
    let refused = service(healthy()).report(ReportScopeRequest::default());

    // A report over every session is not a thing this surface offers, and treating a missing
    // session as "all of them" would answer with everything there is.
    assert_eq!(refused.unwrap_err(), ReportScopeError::MissingSession);
}

#[test]
fn too_many_runs_are_refused_rather_than_clamped() {
    let refused = service(healthy()).report(ReportScopeRequest {
        session_id: SESSION.to_string(),
        run_ids: (0..500).map(|index| format!("run-{index}")).collect(),
        ..ReportScopeRequest::default()
    });

    // Clamping answers a different question under the asked question's name: a caller who
    // requested five hundred runs and received fifty would report on fifty and say five hundred.
    assert_eq!(refused.unwrap_err(), ReportScopeError::TooManyRuns);
}

#[test]
fn an_inverted_range_is_refused() {
    let refused = service(healthy()).report(ReportScopeRequest {
        session_id: SESSION.to_string(),
        from: Some("2026-08-25T10:00:00Z".to_string()),
        to: Some("2026-08-24T10:00:00Z".to_string()),
        ..ReportScopeRequest::default()
    });

    // An inverted range selects nothing, and a report over nothing is indistinguishable from a
    // session that did nothing.
    assert_eq!(refused.unwrap_err(), ReportScopeError::InvalidRange);
}

#[test]
fn an_unreadable_timestamp_is_refused() {
    let refused = service(healthy()).report(ReportScopeRequest {
        session_id: SESSION.to_string(),
        from: Some("last tuesday".to_string()),
        ..ReportScopeRequest::default()
    });

    assert_eq!(refused.unwrap_err(), ReportScopeError::InvalidRange);
}

#[test]
fn an_unrecognised_group_by_is_refused_rather_than_defaulted() {
    let refused = service(healthy()).report(ReportScopeRequest {
        session_id: SESSION.to_string(),
        group_by: Some("phase-of-the-moon".to_string()),
        ..ReportScopeRequest::default()
    });

    // Falling back to `run` would return a report grouped one way under a request that asked for
    // another, and nothing in the response would say so.
    assert_eq!(refused.unwrap_err(), ReportScopeError::InvalidGroupBy);
}

#[test]
fn blank_and_duplicate_ids_are_dropped_rather_than_narrowing_to_nothing() {
    let report = service(healthy())
        .report(ReportScopeRequest {
            session_id: format!("  {SESSION}  "),
            run_ids: vec!["run-1".to_string(), "  ".to_string(), "run-1".to_string()],
            ..ReportScopeRequest::default()
        })
        .expect("report");

    assert_eq!(report.scope.session_id, SESSION);
    // A blank id would narrow to records carrying an empty correlation, which is none of them —
    // one stray empty string would report on nothing and look like a session that did nothing.
    assert_eq!(report.scope.run_ids, vec!["run-1".to_string()]);
}

// ---------------------------------------------------------------------------------------------
// Sections that carry rows
// ---------------------------------------------------------------------------------------------

#[test]
fn a_child_agent_appears_as_its_own_row() {
    let mut sources = healthy();
    sources.runs = Some(RunOutcomeSummary {
        agents: vec![
            AgentReportRow {
                agent_id: Some("claude-code".to_string()),
                run_count: 2,
                ..AgentReportRow::default()
            },
            AgentReportRow {
                agent_id: Some("researcher".to_string()),
                seat_id: Some("seat-2".to_string()),
                run_count: 1,
                ..AgentReportRow::default()
            },
        ],
        ..sources.runs.take().expect("runs")
    });

    let report = service(sources).report(request()).expect("report");

    // A delegated child did its own work and failed or succeeded on its own. Folding it into its
    // parent would make a report unable to answer which agent the failures came from.
    assert_eq!(report.agents.len(), 2);
    assert_eq!(report.agents[1].seat_id.as_deref(), Some("seat-2"));
}

#[test]
fn a_retry_is_counted_without_inflating_the_run_count() {
    let report = service(healthy()).report(request()).expect("report");

    // Three runs, one of which was a retry. Counting the retry as a fourth run would report more
    // work than happened.
    assert_eq!(report.overview.run_count, 3);
    assert_eq!(report.overview.retries, 1);
}

#[test]
fn a_test_failure_is_reported_beside_what_passed() {
    let report = service(healthy()).report(request()).expect("report");

    // Two failures out of thirteen is a different fact from two failures alone, and a report that
    // showed only the failures would make every run look broken.
    assert_eq!(report.verification.failed, 2);
    assert_eq!(report.verification.passed, 10);
    assert_eq!(report.verification.skipped, 1);
}

#[test]
fn changed_files_are_counted_beside_what_is_still_unviewed() {
    let report = service(healthy()).report(request()).expect("report");

    assert_eq!(report.changes.changed_files, 7);
    assert_eq!(report.changes.unviewed_files, 2);
    assert_eq!(report.changes.unresolved_findings, 1);
}

#[test]
fn failures_from_the_logs_join_the_ones_from_the_evidence() {
    let mut sources = healthy();
    sources.logs = Some(LogFailureSummary {
        rows: vec![FailureReportRow {
            reason_code: "log_source_unreadable".to_string(),
            count: 3,
            target: None,
        }],
        incomplete: false,
    });

    let report = service(sources).report(request()).expect("report");

    // A reader looking for what went wrong does not care which subsystem noticed it.
    let codes: Vec<&str> = report
        .failures
        .iter()
        .map(|row| row.reason_code.as_str())
        .collect();
    assert!(codes.contains(&"tool_timeout"));
    assert!(codes.contains(&"log_source_unreadable"));
}

#[test]
fn a_truncated_failure_list_says_so_rather_than_claiming_to_be_all_of_them() {
    let mut sources = healthy();
    sources.logs = Some(LogFailureSummary {
        rows: (0..MAX_FAILURE_ROWS + 10)
            .map(|index| FailureReportRow {
                reason_code: format!("reason_{index}"),
                count: 1,
                target: None,
            })
            .collect(),
        incomplete: false,
    });

    let report = service(sources).report(request()).expect("report");

    assert_eq!(report.failures.len(), MAX_FAILURE_ROWS);
    // A truncated list claiming completeness is the same false claim as a zero for a missing
    // measurement.
    assert_eq!(
        report.coverage.sections["failures"].state,
        ReportCoverageState::Partial
    );
}

#[test]
fn evidence_links_are_offered_even_when_a_section_could_not_answer() {
    let report = service(Sources::default())
        .report(request())
        .expect("report");

    // A reader whose section is unavailable is precisely the one who needs to go and look, so the
    // links come from the scope rather than from the results.
    assert!(!report.evidence_links.is_empty());
    assert!(report
        .evidence_links
        .iter()
        .all(|link| link.session_id == SESSION));
}

// ---------------------------------------------------------------------------------------------
// The tokens a caller reads
// ---------------------------------------------------------------------------------------------

/// Every group-by token parses back to what emitted it.
///
/// A renderer sends a token and reads one back. If the two spellings ever drift the request is
/// refused as an unknown dimension, which reads to a user as a broken control rather than as a
/// mismatch between two lists.
#[test]
fn every_group_by_token_round_trips() {
    for dimension in [
        ReportGroupBy::Run,
        ReportGroupBy::Agent,
        ReportGroupBy::Seat,
        ReportGroupBy::Model,
        ReportGroupBy::Tool,
    ] {
        assert_eq!(ReportGroupBy::parse(dimension.token()), Some(dimension));
    }
}

#[test]
fn coverage_states_carry_the_same_tokens_as_the_rest_of_the_console() {
    // The logs and traces surfaces already publish these four spellings. A report inventing its own
    // would make one console show two vocabularies for one idea.
    assert_eq!(ReportCoverageState::Complete.token(), "complete");
    assert_eq!(ReportCoverageState::Indexing.token(), "indexing");
    assert_eq!(ReportCoverageState::Partial.token(), "partial");
    assert_eq!(ReportCoverageState::Unavailable.token(), "unavailable");
}

#[test]
fn every_refusal_carries_a_distinct_stable_code() {
    let codes: Vec<&str> = [
        ReportScopeError::MissingSession,
        ReportScopeError::TooManyRuns,
        ReportScopeError::TooManySeats,
        ReportScopeError::InvalidRange,
        ReportScopeError::InvalidGroupBy,
    ]
    .into_iter()
    .map(ReportScopeError::code)
    .collect();

    // A caller decides what to say to the user from the code. Two refusals sharing one would make
    // "you asked for too many runs" and "your dates are backwards" indistinguishable.
    let mut distinct = codes.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        codes.len(),
        "{codes:?} are not all distinct"
    );
}

#[test]
fn a_report_carries_no_message_or_command_text() {
    let report = service(healthy()).report(request()).expect("report");

    // Every failure is a stable code. A report is quoted, and a message quoted out of one is
    // producer text in a document nobody redacted.
    for row in &report.failures {
        assert!(
            !row.reason_code.contains(' '),
            "{} reads as prose",
            row.reason_code
        );
    }
}
