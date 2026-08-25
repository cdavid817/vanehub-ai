//! The session-run report's wire contract.
//!
//! Field names and shapes match the Zod schemas the frontend committed to in Task Group 2, because
//! activation is proven by re-running those conformance cases against these payloads rather than by
//! new tests written to fit whatever this layer happens to emit.
//!
//! Every optional field here is optional for the same reason it is optional in the application
//! model: the figure could not be derived. `skip_serializing_if` rather than `null`, so an absent
//! measurement is missing from the payload instead of arriving as a value the UI has to special-case.

use serde::{Deserialize, Serialize};

/// Where a report section sends a reader.
///
/// Identifiers and a tab name, nested exactly as the frontend's target schema expects. Nothing it
/// points at travels with it: a link that carried its target would make the report a second copy of
/// the evidence it summarises, redacted by a different code path.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceTargetScopeDto {
    pub(crate) session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) seat_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) span_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) operation_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceEvidenceTargetDto {
    pub(crate) tab: String,
    pub(crate) scope: EvidenceTargetScopeDto,
}

/// What an export did, in the same two-state shape the session and log exports already use.
///
/// `cancelled` carries no path because there is no file. A result that reported a path for a
/// dismissed picker would name a file nobody could open.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionRunReportExportDto {
    pub(crate) status: &'static str,
    pub(crate) path: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionRunReportRequestDto {
    pub(crate) session_id: Option<String>,
    pub(crate) run_ids: Option<Vec<String>>,
    pub(crate) seat_ids: Option<Vec<String>>,
    pub(crate) from: Option<String>,
    pub(crate) to: Option<String>,
    pub(crate) group_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportScopeDto {
    pub(crate) session_id: String,
    pub(crate) run_ids: Vec<String>,
    pub(crate) seat_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) to: Option<String>,
    pub(crate) group_by: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportSectionCoverageDto {
    pub(crate) state: String,
    pub(crate) reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportCoverageSectionsDto {
    pub(crate) overview: ReportSectionCoverageDto,
    pub(crate) usage: ReportSectionCoverageDto,
    pub(crate) latency: ReportSectionCoverageDto,
    pub(crate) agents: ReportSectionCoverageDto,
    pub(crate) tools: ReportSectionCoverageDto,
    pub(crate) commands: ReportSectionCoverageDto,
    pub(crate) changes: ReportSectionCoverageDto,
    pub(crate) verification: ReportSectionCoverageDto,
    pub(crate) failures: ReportSectionCoverageDto,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportCoverageDto {
    pub(crate) overall: String,
    pub(crate) sections: ReportCoverageSectionsDto,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportOverviewDto {
    pub(crate) run_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) duration_ms: Option<u64>,
    pub(crate) succeeded: u32,
    pub(crate) failed: u32,
    pub(crate) cancelled: u32,
    pub(crate) retries: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionUsageReportDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reported_input_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reported_output_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reported_derived_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) estimated_characters: Option<i64>,
    pub(crate) response_count: u32,
    pub(crate) internal_purpose_response_count: u32,
    pub(crate) coverage: ReportSectionCoverageDto,
    /// Always `false`, and the frontend schema pins it to the literal.
    ///
    /// A backend that started sending `true` would be asserting a figure no versioned pricing
    /// observation backs; the literal makes that a parse failure rather than a number the Report tab
    /// would display.
    pub(crate) cost_available: bool,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LatencyReportDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) p50_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) p95_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) slowest_record_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentReportRowDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) seat_id: Option<String>,
    pub(crate) run_count: u32,
    pub(crate) failed_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolReportRowDto {
    pub(crate) tool_name: String,
    pub(crate) invocations: u32,
    pub(crate) failures: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandReportDto {
    pub(crate) total: u32,
    pub(crate) failed: u32,
    pub(crate) running: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChangeReportDto {
    pub(crate) changed_files: u32,
    /// Absent in this build: nothing records per-file review progress, and zero would claim every
    /// changed file had been looked at.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) unviewed_files: Option<u32>,
    pub(crate) unresolved_findings: u32,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VerificationReportDto {
    pub(crate) passed: u32,
    pub(crate) failed: u32,
    pub(crate) skipped: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FailureReportRowDto {
    pub(crate) reason_code: String,
    pub(crate) count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) target: Option<WorkspaceEvidenceTargetDto>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FailureReportDto {
    pub(crate) rows: Vec<FailureReportRowDto>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionRunReportDto {
    pub(crate) scope: ReportScopeDto,
    pub(crate) generated_at: String,
    pub(crate) coverage: ReportCoverageDto,
    pub(crate) overview: ReportOverviewDto,
    pub(crate) usage: SessionUsageReportDto,
    pub(crate) latency: LatencyReportDto,
    pub(crate) agents: Vec<AgentReportRowDto>,
    pub(crate) tools: Vec<ToolReportRowDto>,
    pub(crate) commands: CommandReportDto,
    pub(crate) changes: ChangeReportDto,
    pub(crate) verification: VerificationReportDto,
    pub(crate) failures: FailureReportDto,
    pub(crate) evidence_links: Vec<WorkspaceEvidenceTargetDto>,
    /// The read as a whole, in the same vocabulary every other evidence answer uses.
    ///
    /// Derived from the weakest section rather than tracked separately: a report is exactly as
    /// trustworthy as its least trustworthy part, and a second source of truth for that would be one
    /// more thing that could disagree with the sections beside it.
    pub(crate) source_coverage: super::evidence_dto::QueryCoverageDto,
}
