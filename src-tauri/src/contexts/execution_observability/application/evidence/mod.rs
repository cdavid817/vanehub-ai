//! Evidence use cases and the ports they depend on.
//!
//! Submodules stay reachable inside the crate so the repository and command layers can name the
//! models they map, while the re-exports below carry the names the rest of the crate uses today.
pub(crate) mod models;
pub(crate) mod ports;
pub(crate) mod report_models;
pub(crate) mod service;

#[cfg(test)]
pub(crate) use models::EvidenceCorrelationCounts;
pub(crate) use models::{
    EvidenceRecordPage, EvidenceSubscriptionBootstrap, ExecutionRecordDetailQuery,
    ExecutionRecordDetailView, ExecutionRecordQuery, RecordEvidenceOutcome,
    WorkspaceEvidenceSummary, WorkspaceEvidenceSummaryQuery,
};
pub(crate) use ports::{
    EvidenceApplicationError, EvidenceClockPort, EvidenceGapDiagnosticsPort,
    EvidenceIdGeneratorPort, EvidenceRedactionValidatorPort, EvidenceRepositoryPort,
    EvidenceRetentionSummary, PostCommitEvidenceNoticePublisherPort,
};
pub(crate) use report_models::{
    EvidenceLatencyAggregate, EvidenceReportAggregate, EvidenceReportQuery,
};
pub(crate) use service::{ExecutionEvidenceService, ProjectionRepair, RecordEvidenceInput};

#[cfg(test)]
mod tests;
