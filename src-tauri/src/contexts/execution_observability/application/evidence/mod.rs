//! Evidence use cases and the ports they depend on.
//!
//! Submodules stay reachable inside the crate so the repository and command layers can name the
//! models they map, while the re-exports below carry the names the rest of the crate uses today.
pub(crate) mod models;
pub(crate) mod ports;
pub(crate) mod service;

pub(crate) use models::{
    EvidenceCorrelationCounts, EvidenceNotice, EvidenceNoticeKind, EvidenceQueryScope,
    EvidenceRecordPage, EvidenceSubscriptionBootstrap, ExecutionRecordDetailFields,
    ExecutionRecordDetailQuery, ExecutionRecordDetailView, ExecutionRecordFilters,
    ExecutionRecordKind, ExecutionRecordProjection, ExecutionRecordQuery, RecordEvidenceOutcome,
    UnownedSummarySource, WorkspaceEvidenceSummary, WorkspaceEvidenceSummaryQuery,
    DEFAULT_EVIDENCE_PAGE_SIZE, MAX_EVIDENCE_PAGE_SIZE,
};
pub(crate) use ports::{
    EvidenceAppendOutcome, EvidenceApplicationError, EvidenceClockPort, EvidenceGapDiagnosticsPort,
    EvidenceIdGeneratorPort, EvidenceRedactionValidatorPort, EvidenceRepositoryPort,
    PostCommitEvidenceNoticePublisherPort,
};
pub(crate) use service::{
    bounded_page_size, capture_not_initialized_coverage, ExecutionEvidenceService,
    RecordEvidenceInput,
};

#[cfg(test)]
mod tests;
