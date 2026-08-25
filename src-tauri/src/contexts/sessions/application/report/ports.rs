//! What the report needs from the contexts that did the work.
//!
//! Five narrow ports rather than five context APIs. The distinction matters because a report is
//! the one place tempted to reach everywhere: it summarises evidence, traces, logs, review state
//! and usage, and a version of it that imported each of those contexts directly would make the
//! sessions context depend on all of them — after which nothing could be changed without changing
//! the report too.
//!
//! Each port is named for the question the report asks, in the sessions context's own vocabulary,
//! and answers with a summary rather than with rows. That is deliberate: a port returning records
//! would put the aggregation in the report, and the aggregation is exactly what the owning context
//! already knows how to do correctly.
//!
//! Every one of them can fail, and failure is not zero. A section whose source could not answer is
//! `unavailable`, and the report says so — the alternative is a zero in a document somebody quotes.

use super::scope::ReportScope;

/// What the report could not get, as a stable code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReportSourceError {
    Unavailable(&'static str),
}

impl ReportSourceError {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Unavailable(code) => code,
        }
    }
}

pub(crate) type ReportSourceResult<T> = Result<T, ReportSourceError>;

/// Run outcomes and per-agent rows, from whoever recorded the executions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RunOutcomeSummary {
    pub(crate) run_count: u32,
    pub(crate) succeeded: u32,
    pub(crate) failed: u32,
    pub(crate) cancelled: u32,
    pub(crate) retries: u32,
    /// Absent when any run in scope is unfinished.
    pub(crate) total_duration_ms: Option<u64>,
    pub(crate) agents: Vec<super::models::AgentReportRow>,
    /// True when the source knows its own answer is short — an indexing projection, a dropped
    /// record. The report turns this into `partial` rather than deciding for itself.
    pub(crate) incomplete: bool,
}

/// Tool and command activity, and what failed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ExecutionEvidenceSummary {
    pub(crate) tools: Vec<super::models::ToolReportRow>,
    pub(crate) commands: super::models::CommandReport,
    pub(crate) verification: super::models::VerificationReport,
    pub(crate) failures: Vec<super::models::FailureReportRow>,
    pub(crate) incomplete: bool,
}

/// Timings, from whoever timed them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TimingSummary {
    pub(crate) p50_ms: Option<u64>,
    pub(crate) p95_ms: Option<u64>,
    pub(crate) slowest_record_duration_ms: Option<u64>,
    pub(crate) incomplete: bool,
}

/// What the logs say went wrong, as counted reason codes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LogFailureSummary {
    pub(crate) rows: Vec<super::models::FailureReportRow>,
    pub(crate) incomplete: bool,
}

/// Review and workspace state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ChangeSummary {
    pub(crate) changed_files: u32,
    pub(crate) unviewed_files: u32,
    pub(crate) unresolved_findings: u32,
    pub(crate) incomplete: bool,
}

/// Usage, with its three qualities already separated by the context that owns the distinction.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReportUsageSummary {
    pub(crate) reported_input_tokens: Option<i64>,
    pub(crate) reported_output_tokens: Option<i64>,
    pub(crate) reported_derived_tokens: Option<i64>,
    pub(crate) estimated_characters: Option<i64>,
    pub(crate) response_count: u32,
    pub(crate) internal_purpose_response_count: u32,
    pub(crate) incomplete: bool,
}

pub(crate) trait RunOutcomePort: Send + Sync {
    fn run_outcomes(&self, scope: &ReportScope) -> ReportSourceResult<RunOutcomeSummary>;
}

pub(crate) trait ExecutionEvidencePort: Send + Sync {
    fn execution_evidence(
        &self,
        scope: &ReportScope,
    ) -> ReportSourceResult<ExecutionEvidenceSummary>;
}

pub(crate) trait ObservabilityTimingPort: Send + Sync {
    fn timings(&self, scope: &ReportScope) -> ReportSourceResult<TimingSummary>;
}

pub(crate) trait LogFailurePort: Send + Sync {
    fn log_failures(&self, scope: &ReportScope) -> ReportSourceResult<LogFailureSummary>;
}

pub(crate) trait ChangeSummaryPort: Send + Sync {
    fn changes(&self, scope: &ReportScope) -> ReportSourceResult<ChangeSummary>;
}

pub(crate) trait ReportUsagePort: Send + Sync {
    fn usage(&self, scope: &ReportScope) -> ReportSourceResult<ReportUsageSummary>;
}

pub(crate) trait ReportClock: Send + Sync {
    fn now(&self) -> String;
}
