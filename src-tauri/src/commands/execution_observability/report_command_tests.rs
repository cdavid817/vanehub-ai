//! What the registered report command actually puts on the wire.
//!
//! The handler body is driven directly, so these assert the payload the frontend will receive rather
//! than a hand-written fixture that resembles it. The frontend's own conformance suite proves its
//! parser accepts a payload of the declared shape; it says nothing about whether anything produces
//! one, which is what this file is for.

use super::export_session_run_report::export_report;
use super::get_session_run_report::session_run_report;
use super::report_dto::SessionRunReportRequestDto;
use crate::contexts::sessions::api::{
    AgentReportRow, ChangeSummary, ChangeSummaryPort, CommandReport, ExecutionEvidencePort,
    ExecutionEvidenceSummary, FailureReportRow, LogFailurePort, LogFailureSummary,
    ObservabilityTimingPort, ReportClock, ReportExportPort, ReportScope, ReportSourceError,
    ReportSourceResult, ReportUsagePort, ReportUsageSummary, RunOutcomePort, RunOutcomeSummary,
    SessionRunReportService, TimingSummary, ToolReportRow, VerificationReport,
};
use serde_json::Value;
use std::sync::{Arc, Mutex};

const SESSION: &str = "session-1";

#[derive(Default)]
struct Sources {
    healthy: bool,
}

impl RunOutcomePort for Sources {
    fn run_outcomes(&self, _scope: &ReportScope) -> ReportSourceResult<RunOutcomeSummary> {
        if !self.healthy {
            return Err(ReportSourceError::Unavailable("runs_unavailable"));
        }
        Ok(RunOutcomeSummary {
            run_count: 2,
            succeeded: 1,
            failed: 1,
            retries: 1,
            total_duration_ms: Some(71_000),
            agents: vec![AgentReportRow {
                agent_id: Some("agent-1".to_string()),
                seat_id: Some("seat-builder".to_string()),
                run_count: 2,
                failed_count: 1,
                duration_ms: Some(71_000),
            }],
            ..RunOutcomeSummary::default()
        })
    }
}

impl ExecutionEvidencePort for Sources {
    fn execution_evidence(
        &self,
        _scope: &ReportScope,
    ) -> ReportSourceResult<ExecutionEvidenceSummary> {
        if !self.healthy {
            return Err(ReportSourceError::Unavailable("evidence_unavailable"));
        }
        Ok(ExecutionEvidenceSummary {
            tools: vec![ToolReportRow {
                tool_name: "read_file".to_string(),
                invocations: 3,
                failures: 0,
                // Absent on purpose: this is the field whose omission the wire contract has to
                // carry, and a payload that serialized `null` would parse as a present value.
                duration_ms: None,
            }],
            commands: CommandReport {
                total: 1,
                failed: 1,
                running: 0,
                duration_ms: Some(12_400),
            },
            verification: VerificationReport {
                passed: 138,
                failed: 2,
                skipped: 0,
            },
            failures: vec![FailureReportRow {
                reason_code: "command_failed_exit".to_string(),
                count: 1,
                target: None,
            }],
            incomplete: false,
        })
    }
}

impl ObservabilityTimingPort for Sources {
    fn timings(&self, _scope: &ReportScope) -> ReportSourceResult<TimingSummary> {
        if !self.healthy {
            return Err(ReportSourceError::Unavailable("timings_unavailable"));
        }
        Ok(TimingSummary {
            p50_ms: Some(31),
            p95_ms: Some(12_400),
            slowest_record_duration_ms: Some(12_400),
            incomplete: false,
        })
    }
}

impl LogFailurePort for Sources {
    fn log_failures(&self, _scope: &ReportScope) -> ReportSourceResult<LogFailureSummary> {
        if !self.healthy {
            return Err(ReportSourceError::Unavailable("logs_unavailable"));
        }
        Ok(LogFailureSummary::default())
    }
}

impl ChangeSummaryPort for Sources {
    fn changes(&self, _scope: &ReportScope) -> ReportSourceResult<ChangeSummary> {
        if !self.healthy {
            return Err(ReportSourceError::Unavailable("changes_unavailable"));
        }
        Ok(ChangeSummary {
            changed_files: 8,
            unviewed_files: None,
            unresolved_findings: 2,
            incomplete: false,
        })
    }
}

impl ReportUsagePort for Sources {
    fn usage(&self, _scope: &ReportScope) -> ReportSourceResult<ReportUsageSummary> {
        if !self.healthy {
            return Err(ReportSourceError::Unavailable("usage_unavailable"));
        }
        Ok(ReportUsageSummary {
            reported_input_tokens: Some(90_000),
            reported_output_tokens: Some(22_000),
            reported_derived_tokens: None,
            estimated_characters: Some(40_000),
            response_count: 3,
            internal_purpose_response_count: 1,
            incomplete: false,
        })
    }
}

/// Remembers what it was asked to write instead of touching a disk.
///
/// The property under test is the filename and the refusal, not the bytes landing anywhere: the
/// bounded write itself belongs to the adapter the session export already proves.
#[derive(Default)]
struct RecordingExport {
    writes: Mutex<Vec<(String, String, String)>>,
    refuse: bool,
}

impl ReportExportPort for RecordingExport {
    fn write_export(
        &self,
        destination_directory: &str,
        filename: &str,
        content: &str,
    ) -> ReportSourceResult<String> {
        if self.refuse {
            return Err(ReportSourceError::Unavailable("report_export_failed"));
        }
        self.writes.lock().expect("writes").push((
            destination_directory.to_string(),
            filename.to_string(),
            content.to_string(),
        ));
        Ok(format!("{destination_directory}/{filename}"))
    }
}

struct FixedClock;

impl ReportClock for FixedClock {
    fn now(&self) -> String {
        "2026-08-25T10:00:00Z".to_string()
    }
}

fn service(healthy: bool) -> SessionRunReportService {
    service_with_export(healthy, Arc::new(RecordingExport::default()))
}

fn service_with_export(healthy: bool, exports: Arc<RecordingExport>) -> SessionRunReportService {
    let sources = Arc::new(Sources { healthy });
    SessionRunReportService::new(
        sources.clone(),
        sources.clone(),
        sources.clone(),
        sources.clone(),
        sources.clone(),
        sources,
        exports,
        Arc::new(FixedClock),
    )
}

fn request() -> SessionRunReportRequestDto {
    SessionRunReportRequestDto {
        session_id: Some(SESSION.to_string()),
        ..SessionRunReportRequestDto::default()
    }
}

fn payload(healthy: bool) -> Value {
    let dto = session_run_report(&service(healthy), request()).expect("report");
    serde_json::to_value(dto).expect("serialize")
}

#[test]
fn the_payload_carries_every_section_the_schema_requires() {
    let value = payload(true);

    for key in [
        "scope",
        "generatedAt",
        "coverage",
        "overview",
        "usage",
        "latency",
        "agents",
        "tools",
        "commands",
        "changes",
        "verification",
        "failures",
        "evidenceLinks",
        "sourceCoverage",
    ] {
        assert!(value.get(key).is_some(), "missing {key}");
    }
    for section in [
        "overview",
        "usage",
        "latency",
        "agents",
        "tools",
        "commands",
        "changes",
        "verification",
        "failures",
    ] {
        assert!(
            value["coverage"]["sections"][section]["state"].is_string(),
            "missing coverage for {section}"
        );
    }
}

/// An absent figure is missing from the payload rather than present as `null`.
#[test]
fn an_underivable_figure_is_omitted_rather_than_serialized_as_null() {
    let value = payload(true);

    // `null` would parse as a present value under a schema that only marks the field optional, and
    // a UI reading it would render the absence as a measurement it can format.
    assert!(value["tools"][0].get("durationMs").is_none());
    assert!(value["changes"].get("unviewedFiles").is_none());
    assert!(value["usage"].get("reportedDerivedTokens").is_none());
    // Present figures still arrive.
    assert_eq!(value["commands"]["durationMs"], 12_400);
    assert_eq!(value["changes"]["changedFiles"], 8);
}

#[test]
fn the_payload_never_claims_a_monetary_cost() {
    for healthy in [true, false] {
        assert_eq!(
            payload(healthy)["usage"]["costAvailable"],
            Value::Bool(false)
        );
    }
}

/// Every tab a link names exists in the frontend's tab enum.
///
/// Read out of the contract file rather than duplicated here, because a link naming a tab that does
/// not exist is a dead link neither side's type system would catch: the tab is a string on the wire.
#[test]
fn every_evidence_link_names_a_tab_the_frontend_has() {
    let contract = include_str!("../../../../src/contracts/session-workspace-evidence-core.ts");
    let value = payload(true);

    let links = value["evidenceLinks"].as_array().expect("links");
    assert!(!links.is_empty());
    for link in links {
        let tab = link["tab"].as_str().expect("tab");
        assert!(
            contract.contains(&format!("\"{tab}\",")),
            "{tab} is not in workspaceEvidenceTabIdSchema"
        );
        assert_eq!(link["scope"]["sessionId"], SESSION);
    }
}

/// An unavailable source degrades a section without emptying the payload.
#[test]
fn an_unavailable_report_still_carries_its_sections_and_says_why() {
    let value = payload(false);

    assert_eq!(value["coverage"]["overall"], "unavailable");
    assert_eq!(
        value["coverage"]["sections"]["usage"]["reasonCodes"][0],
        "usage_unavailable"
    );
    // The zeroes beside it are the defaults, and the coverage is the only thing that says so.
    assert_eq!(value["overview"]["runCount"], 0);
    assert!(value["overview"].get("durationMs").is_none());
    // The links still point somewhere: a reader whose section is unavailable is precisely the one
    // who needs to go and look.
    assert!(!value["evidenceLinks"].as_array().expect("links").is_empty());
}

#[test]
fn the_source_coverage_gathers_the_reasons_its_sections_gave() {
    let value = payload(false);

    let reasons = value["sourceCoverage"]["reasonCodes"]
        .as_array()
        .expect("reason codes");
    assert!(reasons.iter().any(|code| code == "usage_unavailable"));
    assert_eq!(value["sourceCoverage"]["state"], "unavailable");
    // A report is bounded by refusing an over-large scope, never by cutting one short, so there is
    // no continuation a reader could be missing.
    assert_eq!(value["sourceCoverage"]["truncated"], Value::Bool(false));
}

// ---------------------------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------------------------

#[test]
fn a_request_without_a_session_is_refused_with_a_reason_code() {
    let error = session_run_report(&service(true), SessionRunReportRequestDto::default())
        .expect_err("refusal");

    assert_eq!(error.reason_code, "report_session_required");
}

#[test]
fn an_over_large_run_list_is_refused_rather_than_clamped() {
    let error = session_run_report(
        &service(true),
        SessionRunReportRequestDto {
            run_ids: Some((0..500).map(|index| format!("run-{index}")).collect()),
            ..request()
        },
    )
    .expect_err("refusal");

    assert_eq!(error.reason_code, "report_too_many_runs");
}

#[test]
fn an_unrecognised_group_by_is_refused_rather_than_defaulted() {
    let error = session_run_report(
        &service(true),
        SessionRunReportRequestDto {
            group_by: Some("phase-of-the-moon".to_string()),
            ..request()
        },
    )
    .expect_err("refusal");

    assert_eq!(error.reason_code, "report_invalid_group_by");
}

#[test]
fn the_scope_echoes_what_was_actually_used() {
    let value = serde_json::to_value(
        session_run_report(
            &service(true),
            SessionRunReportRequestDto {
                run_ids: Some(vec!["run-1".to_string(), "  ".to_string()]),
                group_by: Some("agent".to_string()),
                ..request()
            },
        )
        .expect("report"),
    )
    .expect("serialize");

    // A caller compares what it asked for against what it got. A scope that echoed the request
    // rather than the validated result would hide the dropped blank.
    assert_eq!(
        value["scope"]["runIds"].as_array().expect("run ids").len(),
        1
    );
    assert_eq!(value["scope"]["groupBy"], "agent");
}

// ---------------------------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------------------------

/// The exported bytes are the payload, not a second rendering of it.
///
/// An export that serialized its own structure would be free to drift from the report on screen,
/// and the drift would only be noticed by somebody comparing a saved file against a panel they no
/// longer have open.
#[test]
fn the_exported_json_is_the_same_payload_the_read_returns() {
    let exports = Arc::new(RecordingExport::default());
    let service = service_with_export(true, exports.clone());

    let result = export_report(&service, request(), "D:/exports".to_string()).expect("export");

    assert_eq!(result.status, "exported");
    let writes = exports.writes.lock().expect("writes");
    let written: Value = serde_json::from_str(&writes[0].2).expect("written json");
    assert_eq!(written, payload(true));
}

#[test]
fn the_filename_is_derived_rather_than_supplied() {
    let exports = Arc::new(RecordingExport::default());
    let service = service_with_export(true, exports.clone());

    export_report(&service, request(), "D:/exports".to_string()).expect("export");

    let writes = exports.writes.lock().expect("writes");
    // Derived from what the export is of and when it was taken. A caller that chose the name could
    // aim the write at a file that already exists.
    assert_eq!(
        writes[0].1,
        "vanehub-report-session-1-2026-08-25T10_00_00Z.json"
    );
    assert_eq!(writes[0].0, "D:/exports");
}

#[test]
fn a_dismissed_picker_is_cancelled_rather_than_failed() {
    let exports = Arc::new(RecordingExport::default());
    let service = service_with_export(true, exports.clone());

    let result = export_report(&service, request(), String::new()).expect("export");

    // Choosing not to export is a choice. Reporting it as an error would put an alert in front of
    // somebody who pressed Escape.
    assert_eq!(result.status, "cancelled");
    assert_eq!(result.path, None);
    assert!(exports.writes.lock().expect("writes").is_empty());
}

#[test]
fn a_failed_write_is_reported_as_cancelled_with_no_path() {
    let exports = Arc::new(RecordingExport {
        refuse: true,
        ..RecordingExport::default()
    });
    let service = service_with_export(true, exports);

    let result = export_report(&service, request(), "D:/exports".to_string()).expect("export");

    // The request was fine, so it is not refused; there is no file, so there is no path. A caller
    // distinguishes the two by the absent path rather than by a message.
    assert_eq!(result.status, "cancelled");
    assert_eq!(result.path, None);
}

#[test]
fn an_over_large_scope_is_refused_before_anything_is_written() {
    let exports = Arc::new(RecordingExport::default());
    let service = service_with_export(true, exports.clone());

    let error = export_report(
        &service,
        SessionRunReportRequestDto {
            run_ids: Some((0..500).map(|index| format!("run-{index}")).collect()),
            ..request()
        },
        "D:/exports".to_string(),
    )
    .expect_err("refusal");

    assert_eq!(error.reason_code, "report_too_many_runs");
    // Refused before the filesystem, not after a file has been written.
    assert!(exports.writes.lock().expect("writes").is_empty());
}
