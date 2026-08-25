//! The report's six adapters, and the one layer allowed to write them.
//!
//! The sessions context states what a session-run report needs — run outcomes, execution evidence,
//! timings, log failures, changes, usage — as six ports in its own vocabulary. It cannot answer any
//! of them: the answers live in four other contexts, and a sessions context that imported those
//! would depend on all of them, after which none could change without changing the report too.
//!
//! So the translation happens here, where knowing every context is the job rather than a leak. Each
//! adapter below calls a *published* API and maps the result. None of them counts anything: the
//! counting is done by the context that owns the records, because only it can bound the read.
//!
//! Failure is ordinary. Any of the four can be unavailable, and every adapter turns that into
//! `ReportSourceError` rather than into an empty answer — a section that could not be assembled has
//! to be distinguishable from one that found nothing, and that distinction starts here.

use crate::contexts::execution_observability::api::evidence::{
    EvidenceReportQuery, EvidenceSeatId, EvidenceSessionId, ExecutionEvidenceApi,
};
use crate::contexts::execution_observability::api::ExecutionObservabilityApi;
use crate::contexts::execution_observability::domain::{ExecutionStatus, PageRequest};
use crate::contexts::operations::log_api::{
    LogFailureQuery, SessionLogApi, SessionLogCoverageState,
};
use crate::contexts::sessions::api::{
    AgentReportRow, ChangeSummary, ChangeSummaryPort, CommandReport, ExecutionEvidencePort,
    ExecutionEvidenceSummary, FailureReportRow, LogFailurePort, LogFailureSummary,
    MeasurementQuality, ObservabilityTimingPort, ReportClock, ReportScope, ReportSourceError,
    ReportSourceResult, ReportUsagePort, ReportUsageSummary, RunOutcomePort, RunOutcomeSummary,
    SessionRunReportService, SessionsApi, TimingSummary, ToolReportRow, UsagePurpose,
    UsageSummaryQuery, VerificationReport,
};
use crate::contexts::workspaces::api::WorkspaceApi;
use std::collections::BTreeMap;
use std::sync::Arc;

/// How many runs one report page reads.
///
/// `list_runs` is a page, and a report over a session wants all of them. The published page size is
/// the ceiling; a session with more runs than this reports `incomplete` rather than a total that is
/// silently one page's worth.
const REPORT_RUN_PAGE_SIZE: u16 = 100;

/// Builds the report service over the published APIs of every contributing context.
pub(crate) fn assemble_session_run_report(
    evidence: ExecutionEvidenceApi,
    observability: ExecutionObservabilityApi,
    logs: SessionLogApi,
    sessions: SessionsApi,
    workspaces: WorkspaceApi,
) -> SessionRunReportService {
    let runs = Arc::new(RunOutcomeAdapter { observability });
    let evidence = Arc::new(EvidenceAdapter { evidence });
    let logs = Arc::new(LogFailureAdapter { logs });
    let changes = Arc::new(ChangeAdapter {
        sessions: sessions.clone(),
        workspaces,
    });
    let usage = Arc::new(UsageAdapter { sessions });
    SessionRunReportService::new(
        runs,
        evidence.clone(),
        evidence,
        logs,
        changes,
        usage,
        Arc::new(SystemReportClock),
    )
}

struct SystemReportClock;

impl ReportClock for SystemReportClock {
    fn now(&self) -> String {
        chrono::Utc::now().to_rfc3339()
    }
}

/// Run outcomes, from the telemetry the observability context already keeps.
struct RunOutcomeAdapter {
    observability: ExecutionObservabilityApi,
}

impl RunOutcomePort for RunOutcomeAdapter {
    fn run_outcomes(&self, scope: &ReportScope) -> ReportSourceResult<RunOutcomeSummary> {
        let request = PageRequest::new(REPORT_RUN_PAGE_SIZE, None)
            .map_err(|_| ReportSourceError::Unavailable("report_run_page_invalid"))?;
        let page = self
            .observability
            .list_runs(&request, Some(&scope.session_id))
            .map_err(|_| ReportSourceError::Unavailable("runs_unavailable"))?;

        let mut summary = RunOutcomeSummary {
            // A further page means the session has more runs than one read covers, so every count
            // below is a floor rather than a total. The consumer renders that as `partial`.
            incomplete: page.next_page_token.is_some(),
            ..RunOutcomeSummary::default()
        };
        let mut measured_all = true;
        // Keyed rather than indexed so no lookup can fail. Ordering falls out of the key, which
        // makes two reports of the same session list their agents the same way.
        let mut agents: BTreeMap<Option<String>, AgentReportRow> = BTreeMap::new();

        for run in page.items {
            if !scope.run_ids.is_empty()
                && !scope
                    .run_ids
                    .iter()
                    .any(|value| value == run.context.run_id.as_str())
            {
                continue;
            }
            summary.run_count += 1;
            match run.status {
                ExecutionStatus::Succeeded => summary.succeeded += 1,
                ExecutionStatus::Failed => summary.failed += 1,
                ExecutionStatus::Cancelled => summary.cancelled += 1,
                _ => {}
            }

            let duration = run_duration_ms(&run.started_at, run.ended_at.as_deref());
            if duration.is_none() {
                // A run that has not ended has no duration, and a total that skipped it would read
                // as the whole session's elapsed time.
                measured_all = false;
            }

            // A run with no agent id gets its own row rather than being dropped: work that happened
            // under no recorded agent is still work, and a report that omitted it would not add up
            // to its own total.
            let key = run.agent_id.as_deref().map(str::to_string);
            let row = agents.entry(key.clone()).or_insert(AgentReportRow {
                agent_id: key,
                duration_ms: Some(0),
                ..AgentReportRow::default()
            });
            row.run_count += 1;
            if run.status == ExecutionStatus::Failed {
                row.failed_count += 1;
            }
            match (row.duration_ms, duration) {
                (Some(total), Some(value)) => row.duration_ms = Some(total + value),
                _ => row.duration_ms = None,
            }
            if let Some(value) = duration {
                summary.total_duration_ms = Some(summary.total_duration_ms.unwrap_or(0) + value);
            }
        }

        if !measured_all {
            summary.total_duration_ms = None;
        }
        summary.agents = agents.into_values().collect();
        Ok(summary)
    }
}

fn run_duration_ms(started_at: &str, ended_at: Option<&str>) -> Option<u64> {
    let started = chrono::DateTime::parse_from_rfc3339(started_at).ok()?;
    let ended = chrono::DateTime::parse_from_rfc3339(ended_at?).ok()?;
    u64::try_from((ended - started).num_milliseconds()).ok()
}

/// Tools, commands, verifications, failures, and latency — all from the evidence journal.
///
/// One adapter for two ports because both read the same store over the same scope. Splitting them
/// into two types would double the translation without separating anything.
struct EvidenceAdapter {
    evidence: ExecutionEvidenceApi,
}

impl EvidenceAdapter {
    fn query(&self, scope: &ReportScope) -> ReportSourceResult<EvidenceReportQuery> {
        let session_id = EvidenceSessionId::parse(scope.session_id.clone())
            .map_err(|_| ReportSourceError::Unavailable("report_session_unreadable"))?;
        let mut seat_ids = Vec::new();
        for seat in &scope.seat_ids {
            // A seat id the evidence context refuses is dropped rather than failing the section: it
            // cannot match any record, so narrowing by it would report an empty session.
            if let Ok(parsed) = EvidenceSeatId::parse(seat.clone()) {
                seat_ids.push(parsed);
            }
        }
        Ok(EvidenceReportQuery {
            run_ids: scope.run_ids.clone(),
            seat_ids,
            from: scope.from.clone(),
            to: scope.to.clone(),
            ..EvidenceReportQuery::new(session_id)
        })
    }
}

impl ExecutionEvidencePort for EvidenceAdapter {
    fn execution_evidence(
        &self,
        scope: &ReportScope,
    ) -> ReportSourceResult<ExecutionEvidenceSummary> {
        let aggregate = self
            .evidence
            .report_aggregate(&self.query(scope)?)
            .map_err(|_| ReportSourceError::Unavailable("evidence_unavailable"))?;
        Ok(ExecutionEvidenceSummary {
            tools: aggregate
                .tools
                .into_iter()
                .map(|tool| ToolReportRow {
                    tool_name: tool.tool_name,
                    invocations: tool.invocations,
                    failures: tool.failures,
                    duration_ms: tool.duration_ms,
                })
                .collect(),
            commands: CommandReport {
                total: aggregate.commands.total,
                failed: aggregate.commands.failed,
                running: aggregate.commands.running,
                duration_ms: aggregate.commands.duration_ms,
            },
            verification: VerificationReport {
                passed: aggregate.verification.passed,
                failed: aggregate.verification.failed,
                skipped: aggregate.verification.skipped,
            },
            failures: aggregate
                .failures
                .into_iter()
                .map(|failure| FailureReportRow {
                    reason_code: failure.reason_code,
                    count: failure.count,
                    target: None,
                })
                .collect(),
            incomplete: aggregate.incomplete,
        })
    }
}

impl ObservabilityTimingPort for EvidenceAdapter {
    fn timings(&self, scope: &ReportScope) -> ReportSourceResult<TimingSummary> {
        let latency = self
            .evidence
            .report_latency(&self.query(scope)?)
            .map_err(|_| ReportSourceError::Unavailable("timings_unavailable"))?;
        Ok(TimingSummary {
            p50_ms: latency.p50_ms,
            p95_ms: latency.p95_ms,
            slowest_record_duration_ms: latency.slowest_record_duration_ms,
            incomplete: latency.incomplete,
        })
    }
}

/// Error rows from the log index, under one code per category.
struct LogFailureAdapter {
    logs: SessionLogApi,
}

impl LogFailurePort for LogFailureAdapter {
    fn log_failures(&self, scope: &ReportScope) -> ReportSourceResult<LogFailureSummary> {
        let summary = self
            .logs
            .failure_summary(&LogFailureQuery {
                session_id: scope.session_id.clone(),
                run_ids: scope.run_ids.clone(),
                seat_ids: scope.seat_ids.clone(),
                from: scope.from.clone(),
                to: scope.to.clone(),
            })
            .map_err(|_| ReportSourceError::Unavailable("logs_unavailable"))?;
        Ok(LogFailureSummary {
            rows: summary
                .rows
                .into_iter()
                .map(|row| FailureReportRow {
                    // Prefixed so a log-derived row cannot be mistaken for an evidence-derived one
                    // that happens to share a category name. The category is already a stable
                    // token, so the whole code stays groupable.
                    reason_code: format!("log_error:{}", row.category),
                    count: row.count,
                    target: None,
                })
                .collect(),
            // An index that is still building has not seen every error, and a report that showed
            // its partial count as the total would read as a healthier session than it was.
            incomplete: summary.truncated
                || summary.coverage.state() != SessionLogCoverageState::Complete,
        })
    }
}

/// Changed files from the workspace, unresolved findings from the review that already exists.
struct ChangeAdapter {
    sessions: SessionsApi,
    workspaces: WorkspaceApi,
}

impl ChangeSummaryPort for ChangeAdapter {
    fn changes(&self, scope: &ReportScope) -> ReportSourceResult<ChangeSummary> {
        let status = self
            .workspaces
            .get_session_git_status(&scope.session_id)
            .map_err(|_| ReportSourceError::Unavailable("changes_unavailable"))?;
        // A review is read, never opened. `open_review` snapshots and writes, so reporting on a
        // session would otherwise create the review it then described.
        let review = self
            .sessions
            .find_active_review(&scope.session_id)
            .map_err(|_| ReportSourceError::Unavailable("review_unavailable"))?;

        Ok(ChangeSummary {
            changed_files: u32::try_from(status.items.len()).unwrap_or(u32::MAX),
            // Absent rather than zero: nothing in this build records per-file review progress, and
            // zero would claim every changed file had been looked at.
            unviewed_files: None,
            unresolved_findings: review
                .as_ref()
                .map(|review| {
                    review
                        .findings()
                        .iter()
                        .filter(|finding| !finding.resolved)
                        .count()
                })
                .map(|count| u32::try_from(count).unwrap_or(u32::MAX))
                .unwrap_or(0),
            // A truncated status listing is a floor on the changed-file count. A session with no
            // review is not incomplete: having no review is a fact, not a gap.
            incomplete: status.truncated,
        })
    }
}

/// Usage, from the accounting read model that already separates the three qualities.
struct UsageAdapter {
    sessions: SessionsApi,
}

impl ReportUsagePort for UsageAdapter {
    fn usage(&self, scope: &ReportScope) -> ReportSourceResult<ReportUsageSummary> {
        let summary = self
            .sessions
            .token_usage_summary(&UsageSummaryQuery {
                session_id: Some(scope.session_id.clone()),
                range_start: scope.from.clone(),
                range_end: scope.to.clone(),
                // No breakdowns: the report needs totals, and asking for per-dimension entries
                // would pay for rows nothing here reads.
                breakdown_limit: 0,
                generated_at: chrono::Utc::now().to_rfc3339(),
                ..usage_query_defaults()
            })
            .map_err(|_| ReportSourceError::Unavailable("usage_unavailable"))?;

        Ok(ReportUsageSummary {
            // Carried across one quality at a time. The accounting model already keeps them apart,
            // and the one thing this adapter must not do is bring them back together.
            // `observation_count` is what separates "the provider reported zero" from "nobody
            // reported anything". The dimension totals are plain integers and read the same either
            // way, so a session with no accounting at all would otherwise report a measured zero.
            reported_input_tokens: measured(
                summary.totals.reported.observation_count,
                summary.totals.reported.dimensions.input,
            ),
            reported_output_tokens: measured(
                summary.totals.reported.observation_count,
                summary.totals.reported.dimensions.output,
            ),
            reported_derived_tokens: summary.totals.reported_derived.headline_total,
            estimated_characters: summary.totals.estimated.headline_total,
            response_count: u32::try_from(summary.user_response.reported.call_count.max(0))
                .unwrap_or(u32::MAX),
            internal_purpose_response_count: u32::try_from(
                summary.internal.reported.call_count.max(0),
            )
            .unwrap_or(u32::MAX),
            incomplete: false,
        })
    }
}

/// A figure only when something was actually observed.
fn measured(observation_count: i64, value: i64) -> Option<i64> {
    (observation_count > 0).then_some(value)
}

/// The filters a report never narrows by, stated once.
///
/// Spelled out rather than defaulted so that a new filter added to the query cannot silently start
/// applying to reports — the compiler names it here instead.
fn usage_query_defaults() -> UsageSummaryQuery {
    UsageSummaryQuery {
        session_id: None,
        message_id: None,
        generation_id: None,
        agent_id: None,
        provider_id: None,
        model_id: None,
        purpose: None::<UsagePurpose>,
        quality: None::<MeasurementQuality>,
        status: None,
        range_start: None,
        range_end: None,
        breakdown_limit: 0,
        generated_at: String::new(),
    }
}
