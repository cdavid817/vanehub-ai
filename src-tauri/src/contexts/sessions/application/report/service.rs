//! Assembling a report from five sources that can each fail independently.
//!
//! The assembly is mostly about failure. Any source can be unavailable, and the whole design of
//! this type is that one unavailable source degrades one section rather than the report — a report
//! that refused to answer because its change summary was missing would be useless in exactly the
//! situation somebody opens it, and one that answered with zeros would be worse than useless.
//!
//! So each section is filled independently, records its own coverage, and the report's overall
//! state is the weakest of them. Nothing is summed across qualities, nothing missing becomes zero,
//! and no monetary figure appears at all.

use super::models::{
    ChangeReport, CommandReport, LatencyReport, ReportCoverage, ReportEvidenceLink, ReportOverview,
    ReportSectionCoverage, SessionRunReport, SessionUsageReport, VerificationReport,
};
use super::ports::{
    ChangeSummaryPort, ExecutionEvidencePort, LogFailurePort, ObservabilityTimingPort, ReportClock,
    ReportExportPort, ReportSourceError, ReportUsagePort, RunOutcomePort,
};
use super::scope::{validate_report_scope, ReportScope, ReportScopeError, ReportScopeRequest};
use std::sync::Arc;

/// How many failure rows a report carries.
///
/// A report is read, and a hundred distinct reason codes is not something anybody reads — it is a
/// list that hides the three that matter. Truncation is recorded in the section's coverage so the
/// list never silently claims to be all of them.
pub(crate) const MAX_FAILURE_ROWS: usize = 20;

pub(crate) struct SessionRunReportService {
    runs: Arc<dyn RunOutcomePort>,
    evidence: Arc<dyn ExecutionEvidencePort>,
    timings: Arc<dyn ObservabilityTimingPort>,
    logs: Arc<dyn LogFailurePort>,
    changes: Arc<dyn ChangeSummaryPort>,
    usage: Arc<dyn ReportUsagePort>,
    exports: Arc<dyn ReportExportPort>,
    clock: Arc<dyn ReportClock>,
}

impl SessionRunReportService {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        runs: Arc<dyn RunOutcomePort>,
        evidence: Arc<dyn ExecutionEvidencePort>,
        timings: Arc<dyn ObservabilityTimingPort>,
        logs: Arc<dyn LogFailurePort>,
        changes: Arc<dyn ChangeSummaryPort>,
        usage: Arc<dyn ReportUsagePort>,
        exports: Arc<dyn ReportExportPort>,
        clock: Arc<dyn ReportClock>,
    ) -> Self {
        Self {
            runs,
            evidence,
            timings,
            logs,
            changes,
            usage,
            exports,
            clock,
        }
    }

    /// Writes an already-rendered report into a directory the user picked.
    ///
    /// The content is the caller's because only the boundary layer knows the wire shape, and the
    /// wire shape is the point: an export that serialized its own structure would be a second
    /// rendering of the same report, free to drift from the one on screen. The filename is *not*
    /// the caller's — a caller that chose it could aim the write at a file that already exists.
    pub(crate) fn export(
        &self,
        destination_directory: &str,
        session_id: &str,
        content: &str,
    ) -> Result<Option<String>, ReportScopeError> {
        let destination = destination_directory.trim();
        if destination.is_empty() {
            // The picker was dismissed. Not an error: choosing not to export is a choice, and
            // reporting it as a failure would put an alert in front of somebody who pressed Escape.
            return Ok(None);
        }
        let filename = export_filename(session_id, &self.clock.now());
        // A failed write is reported as a cancelled export rather than as a refusal of the request,
        // because the request was fine. The caller distinguishes the two by the absent path.
        Ok(self
            .exports
            .write_export(destination, &filename, content)
            .ok())
    }

    /// One report, or the reason the request itself was refused.
    ///
    /// Scope errors are the only thing that fails the whole call: a request this surface cannot
    /// answer is different from a request it answered incompletely, and collapsing the two would
    /// return an empty report for a malformed query.
    pub(crate) fn report(
        &self,
        request: ReportScopeRequest,
    ) -> Result<SessionRunReport, ReportScopeError> {
        let scope = validate_report_scope(request)?;
        let mut coverage = ReportCoverage::default();

        let (overview, agents) = self.overview_section(&scope, &mut coverage);
        let usage = self.usage_section(&scope, &mut coverage);
        let latency = self.latency_section(&scope, &mut coverage);
        let (tools, commands, verification, mut failures) =
            self.evidence_sections(&scope, &mut coverage);
        let changes = self.change_section(&scope, &mut coverage);

        // Log-derived failures join the evidence-derived ones under one section, because a reader
        // looking for what went wrong does not care which subsystem noticed it.
        let log_failures = self.logs.log_failures(&scope);
        match log_failures {
            Ok(summary) => {
                failures.extend(summary.rows);
                if summary.incomplete {
                    coverage.set(
                        "failures",
                        ReportSectionCoverage::partial("report_source_partial"),
                    );
                }
            }
            Err(error) => {
                coverage.set("failures", ReportSectionCoverage::unavailable(error.code()));
            }
        }
        let truncated = failures.len() > MAX_FAILURE_ROWS;
        failures.truncate(MAX_FAILURE_ROWS);
        if truncated {
            // Recorded rather than silent. A truncated list that claimed to be complete is the
            // same false claim as a zero for a missing measurement.
            coverage.set(
                "failures",
                ReportSectionCoverage::partial("report_failures_truncated"),
            );
        }

        Ok(SessionRunReport {
            generated_at: self.clock.now(),
            evidence_links: evidence_links(&scope),
            coverage,
            overview,
            usage,
            latency,
            agents,
            tools,
            commands,
            changes,
            verification,
            failures,
            scope,
        })
    }

    fn overview_section(
        &self,
        scope: &ReportScope,
        coverage: &mut ReportCoverage,
    ) -> (ReportOverview, Vec<super::models::AgentReportRow>) {
        match self.runs.run_outcomes(scope) {
            Ok(summary) => {
                let state = section_state(summary.incomplete);
                coverage.set("overview", state.clone());
                coverage.set("agents", state);
                (
                    ReportOverview {
                        run_count: summary.run_count,
                        duration_ms: summary.total_duration_ms,
                        succeeded: summary.succeeded,
                        failed: summary.failed,
                        cancelled: summary.cancelled,
                        retries: summary.retries,
                    },
                    summary.agents,
                )
            }
            Err(error) => {
                // Default, not zeroes-that-look-measured: `ReportOverview::default()` carries the
                // same numbers, and the coverage beside it is what says they mean nothing.
                coverage.set("overview", unavailable(error));
                coverage.set("agents", unavailable(error));
                (ReportOverview::default(), Vec::new())
            }
        }
    }

    fn usage_section(
        &self,
        scope: &ReportScope,
        coverage: &mut ReportCoverage,
    ) -> SessionUsageReport {
        match self.usage.usage(scope) {
            Ok(summary) => {
                let state = section_state(summary.incomplete);
                coverage.set("usage", state.clone());
                SessionUsageReport {
                    // Carried across one field at a time and never summed. Adding a reported figure
                    // to a derived one produces a number in no unit at all, and adding either to
                    // an estimate turns the estimate into a measurement.
                    reported_input_tokens: summary.reported_input_tokens,
                    reported_output_tokens: summary.reported_output_tokens,
                    reported_derived_tokens: summary.reported_derived_tokens,
                    estimated_characters: summary.estimated_characters,
                    response_count: summary.response_count,
                    internal_purpose_response_count: summary.internal_purpose_response_count,
                    coverage: state,
                    // No pricing catalog exists in this change, so no cost can be computed from a
                    // rate anybody could check later against what was actually charged.
                    cost_available: false,
                }
            }
            Err(error) => {
                let state = unavailable(error);
                coverage.set("usage", state.clone());
                SessionUsageReport {
                    coverage: state,
                    cost_available: false,
                    ..SessionUsageReport::default()
                }
            }
        }
    }

    fn latency_section(&self, scope: &ReportScope, coverage: &mut ReportCoverage) -> LatencyReport {
        match self.timings.timings(scope) {
            Ok(summary) => {
                coverage.set("latency", section_state(summary.incomplete));
                LatencyReport {
                    p50_ms: summary.p50_ms,
                    p95_ms: summary.p95_ms,
                    slowest_record_duration_ms: summary.slowest_record_duration_ms,
                }
            }
            Err(error) => {
                coverage.set("latency", unavailable(error));
                // Every field absent rather than zero. A p50 of zero would report a session where
                // every call returned instantly.
                LatencyReport::default()
            }
        }
    }

    fn evidence_sections(
        &self,
        scope: &ReportScope,
        coverage: &mut ReportCoverage,
    ) -> (
        Vec<super::models::ToolReportRow>,
        CommandReport,
        VerificationReport,
        Vec<super::models::FailureReportRow>,
    ) {
        match self.evidence.execution_evidence(scope) {
            Ok(summary) => {
                let state = section_state(summary.incomplete);
                for section in ["tools", "commands", "verification", "failures"] {
                    coverage.set(section, state.clone());
                }
                (
                    summary.tools,
                    summary.commands,
                    summary.verification,
                    summary.failures,
                )
            }
            Err(error) => {
                for section in ["tools", "commands", "verification", "failures"] {
                    coverage.set(section, unavailable(error));
                }
                (
                    Vec::new(),
                    CommandReport::default(),
                    VerificationReport::default(),
                    Vec::new(),
                )
            }
        }
    }

    fn change_section(&self, scope: &ReportScope, coverage: &mut ReportCoverage) -> ChangeReport {
        match self.changes.changes(scope) {
            Ok(summary) => {
                coverage.set("changes", section_state(summary.incomplete));
                ChangeReport {
                    changed_files: summary.changed_files,
                    unviewed_files: summary.unviewed_files,
                    unresolved_findings: summary.unresolved_findings,
                }
            }
            Err(error) => {
                coverage.set("changes", unavailable(error));
                ChangeReport::default()
            }
        }
    }
}

/// A filename nobody chose, built from what the export is of and when it was taken.
///
/// Every character outside the safe set is replaced rather than dropped: dropping would let two
/// different session ids collapse onto one name, and a report overwriting another session's report
/// is the failure worth spending a few underscores to avoid.
fn export_filename(session_id: &str, generated_at: &str) -> String {
    let safe = |value: &str| -> String {
        value
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || character == '-' {
                    character
                } else {
                    '_'
                }
            })
            .collect()
    };
    format!(
        "vanehub-report-{}-{}.json",
        safe(session_id),
        safe(generated_at)
    )
}

/// A source that answered but said its answer is short is `partial`, not `complete`.
fn section_state(incomplete: bool) -> ReportSectionCoverage {
    if incomplete {
        ReportSectionCoverage::partial("report_source_partial")
    } else {
        ReportSectionCoverage::complete()
    }
}

fn unavailable(error: ReportSourceError) -> ReportSectionCoverage {
    ReportSectionCoverage::unavailable(error.code())
}

/// Where each section sends a reader who wants the records behind it.
///
/// Built from the scope rather than from the results, so the links are the same whether a section
/// answered or not — a reader whose usage section is unavailable is precisely the one who needs to
/// go and look.
fn evidence_links(scope: &ReportScope) -> Vec<ReportEvidenceLink> {
    let run_id = scope.run_ids.first().cloned();
    let seat_id = scope.seat_ids.first().cloned();
    // Tab tokens the workspace actually has. "review" is not one of them — review lives inside the
    // changes tab, and a link naming a tab that does not exist is a dead link the type system would
    // not have caught on either side of the wire.
    ["terminal-history", "traces", "logs", "changes"]
        .into_iter()
        .map(|tab| ReportEvidenceLink {
            tab: tab.to_string(),
            session_id: scope.session_id.clone(),
            run_id: run_id.clone(),
            seat_id: seat_id.clone(),
            trace_id: None,
            span_id: None,
            operation_id: None,
        })
        .collect()
}
