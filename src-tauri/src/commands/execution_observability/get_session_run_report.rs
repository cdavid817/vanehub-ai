//! The session-run report command: parse, call the published service, map.
//!
//! No aggregation happens here. Every figure in the payload was counted by the context that owns the
//! records it came from, and this handler's whole job is to turn one request into one scope and one
//! report into one wire shape.

use super::{evidence_dto, report_dto};
use crate::contexts::sessions::api::{
    ReportCoverage, ReportCoverageState, ReportEvidenceLink, ReportScopeRequest,
    ReportSectionCoverage, SessionRunReport, SessionRunReportService,
};
use tauri::State;

/// Reads the bounded session-run report.
///
/// Refusals are reason codes, never messages: the frontend translates the code, and a message would
/// arrive untranslated and possibly naming internals.
#[tauri::command]
pub(crate) fn get_session_run_report(
    reports: State<'_, SessionRunReportService>,
    session_id: Option<String>,
    run_ids: Option<Vec<String>>,
    seat_ids: Option<Vec<String>>,
    from: Option<String>,
    to: Option<String>,
    group_by: Option<String>,
) -> Result<report_dto::SessionRunReportDto, evidence_dto::EvidenceCommandErrorDto> {
    session_run_report(
        reports.inner(),
        report_dto::SessionRunReportRequestDto {
            session_id,
            run_ids,
            seat_ids,
            from,
            to,
            group_by,
        },
    )
}

/// The body, separated from the `State` wrapper so tests exercise this code rather than a copy of
/// it. `State` cannot be constructed outside a running app, and a test that re-implemented the
/// handler would pass while the registered one drifted.
pub(super) fn session_run_report(
    reports: &SessionRunReportService,
    request: report_dto::SessionRunReportRequestDto,
) -> Result<report_dto::SessionRunReportDto, evidence_dto::EvidenceCommandErrorDto> {
    let report = reports
        .report(ReportScopeRequest {
            session_id: request.session_id.unwrap_or_default(),
            run_ids: request.run_ids.unwrap_or_default(),
            seat_ids: request.seat_ids.unwrap_or_default(),
            from: request.from,
            to: request.to,
            group_by: request.group_by,
        })
        .map_err(|error| evidence_dto::EvidenceCommandErrorDto {
            reason_code: error.code().to_string(),
        })?;
    Ok(report_dto(&report))
}

fn report_dto(report: &SessionRunReport) -> report_dto::SessionRunReportDto {
    report_dto::SessionRunReportDto {
        scope: report_dto::ReportScopeDto {
            session_id: report.scope.session_id.clone(),
            run_ids: report.scope.run_ids.clone(),
            seat_ids: report.scope.seat_ids.clone(),
            from: report.scope.from.clone(),
            to: report.scope.to.clone(),
            group_by: report.scope.group_by.token().to_string(),
        },
        generated_at: report.generated_at.clone(),
        coverage: coverage_dto(&report.coverage),
        overview: report_dto::ReportOverviewDto {
            run_count: report.overview.run_count,
            duration_ms: report.overview.duration_ms,
            succeeded: report.overview.succeeded,
            failed: report.overview.failed,
            cancelled: report.overview.cancelled,
            retries: report.overview.retries,
        },
        usage: report_dto::SessionUsageReportDto {
            reported_input_tokens: report.usage.reported_input_tokens,
            reported_output_tokens: report.usage.reported_output_tokens,
            reported_derived_tokens: report.usage.reported_derived_tokens,
            estimated_characters: report.usage.estimated_characters,
            response_count: report.usage.response_count,
            internal_purpose_response_count: report.usage.internal_purpose_response_count,
            coverage: section_dto(&report.usage.coverage),
            cost_available: report.usage.cost_available,
        },
        latency: report_dto::LatencyReportDto {
            p50_ms: report.latency.p50_ms,
            p95_ms: report.latency.p95_ms,
            slowest_record_duration_ms: report.latency.slowest_record_duration_ms,
        },
        agents: report
            .agents
            .iter()
            .map(|row| report_dto::AgentReportRowDto {
                agent_id: row.agent_id.clone(),
                seat_id: row.seat_id.clone(),
                run_count: row.run_count,
                failed_count: row.failed_count,
                duration_ms: row.duration_ms,
            })
            .collect(),
        tools: report
            .tools
            .iter()
            .map(|row| report_dto::ToolReportRowDto {
                tool_name: row.tool_name.clone(),
                invocations: row.invocations,
                failures: row.failures,
                duration_ms: row.duration_ms,
            })
            .collect(),
        commands: report_dto::CommandReportDto {
            total: report.commands.total,
            failed: report.commands.failed,
            running: report.commands.running,
            duration_ms: report.commands.duration_ms,
        },
        changes: report_dto::ChangeReportDto {
            changed_files: report.changes.changed_files,
            unviewed_files: report.changes.unviewed_files,
            unresolved_findings: report.changes.unresolved_findings,
        },
        verification: report_dto::VerificationReportDto {
            passed: report.verification.passed,
            failed: report.verification.failed,
            skipped: report.verification.skipped,
        },
        failures: report_dto::FailureReportDto {
            rows: report
                .failures
                .iter()
                .map(|row| report_dto::FailureReportRowDto {
                    reason_code: row.reason_code.clone(),
                    count: row.count,
                    target: row.target.as_ref().map(target_dto),
                })
                .collect(),
        },
        evidence_links: report.evidence_links.iter().map(target_dto).collect(),
        source_coverage: source_coverage_dto(&report.coverage),
    }
}

fn coverage_dto(coverage: &ReportCoverage) -> report_dto::ReportCoverageDto {
    // Read by name rather than by iteration order: the wire shape names its nine sections, and a
    // positional mapping would silently rotate every section if the list were ever reordered.
    let section = |name: &str| {
        coverage
            .sections
            .get(name)
            .map(section_dto)
            .unwrap_or_else(|| section_dto(&ReportSectionCoverage::default()))
    };
    report_dto::ReportCoverageDto {
        overall: coverage.overall().token().to_string(),
        sections: report_dto::ReportCoverageSectionsDto {
            overview: section("overview"),
            usage: section("usage"),
            latency: section("latency"),
            agents: section("agents"),
            tools: section("tools"),
            commands: section("commands"),
            changes: section("changes"),
            verification: section("verification"),
            failures: section("failures"),
        },
    }
}

fn section_dto(section: &ReportSectionCoverage) -> report_dto::ReportSectionCoverageDto {
    report_dto::ReportSectionCoverageDto {
        state: section.state.token().to_string(),
        reason_codes: section.reason_codes.clone(),
    }
}

/// The read as a whole, in the vocabulary every other evidence answer uses.
///
/// `truncated` is false because a report is never a page: it is bounded by refusing an over-large
/// scope rather than by cutting one short, so there is no continuation a reader could be missing.
fn source_coverage_dto(coverage: &ReportCoverage) -> evidence_dto::QueryCoverageDto {
    let overall = coverage.overall();
    let mut reason_codes: Vec<String> = coverage
        .sections
        .values()
        .filter(|section| section.state != ReportCoverageState::Complete)
        .flat_map(|section| section.reason_codes.iter().cloned())
        .collect();
    reason_codes.sort();
    reason_codes.dedup();
    evidence_dto::QueryCoverageDto {
        state: overall.token().to_string(),
        reason_codes,
        oldest_available_at: None,
        newest_available_at: None,
        indexed_through_at: None,
        dropped_count: None,
        truncated: false,
    }
}

fn target_dto(link: &ReportEvidenceLink) -> report_dto::WorkspaceEvidenceTargetDto {
    report_dto::WorkspaceEvidenceTargetDto {
        tab: link.tab.clone(),
        scope: report_dto::EvidenceTargetScopeDto {
            session_id: link.session_id.clone(),
            seat_id: link.seat_id.clone(),
            run_id: link.run_id.clone(),
            trace_id: link.trace_id.clone(),
            span_id: link.span_id.clone(),
            operation_id: link.operation_id.clone(),
        },
    }
}
