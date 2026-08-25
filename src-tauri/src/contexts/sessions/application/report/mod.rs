//! The session-run report, owned by the sessions context.
//!
//! Here rather than in observability or workspaces because the report is about a *session*: it
//! spans every context that did work under one. Any contributor could have hosted it, and each
//! would have made its own signal the centre of it.
//!
//! What crosses this boundary is the service, its ports, and its models. The ports are the reason
//! the arrangement holds: they are named for the questions a report asks, in this context's own
//! vocabulary, and bootstrap supplies adapters over the other contexts' published APIs. Nothing
//! here knows that evidence is SQLite, that timings come from spans, or that changes come from git.

mod models;
mod ports;
mod scope;
mod service;

#[cfg(test)]
mod tests;

// Only what a caller outside this module names. The section models a report *contains* — the
// overview, the usage figures, the group-by dimension — are reached through the report rather than
// constructed, so publishing them would widen the surface without anything asking for it.
pub(crate) use models::{
    AgentReportRow, CommandReport, FailureReportRow, ReportCoverage, ReportCoverageState,
    ReportEvidenceLink, ReportSectionCoverage, SessionRunReport, ToolReportRow, VerificationReport,
};
pub(crate) use ports::{
    ChangeSummary, ChangeSummaryPort, ExecutionEvidencePort, ExecutionEvidenceSummary,
    LogFailurePort, LogFailureSummary, ObservabilityTimingPort, ReportClock, ReportSourceError,
    ReportSourceResult, ReportUsagePort, ReportUsageSummary, RunOutcomePort, RunOutcomeSummary,
    TimingSummary,
};
pub(crate) use scope::{ReportScope, ReportScopeRequest};
pub(crate) use service::SessionRunReportService;
