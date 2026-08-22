use super::models::{
    EvidenceCorrelationCounts, EvidenceNotice, EvidenceRecordPage, EvidenceSubscriptionBootstrap,
    ExecutionRecordDetailQuery, ExecutionRecordDetailView, ExecutionRecordQuery,
    WorkspaceEvidenceSummary, WorkspaceEvidenceSummaryQuery,
};
use crate::contexts::execution_observability::domain::{
    EvidenceSessionId, EvidenceSourceContext, ExecutionEvidenceEvent, SourceEventId,
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum EvidenceApplicationError {
    #[error(transparent)]
    Domain(#[from] crate::contexts::execution_observability::domain::EvidenceDomainError),
    #[error("evidence storage is unavailable: {0}")]
    Storage(String),
    #[error("the supplied cursor does not belong to this query")]
    CursorFilterMismatch,
    #[error("the supplied cursor could not be decoded")]
    InvalidCursor,
    #[error("evidence record was not found")]
    RecordNotFound,
}

/// One ingestion attempt's result at the storage boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EvidenceAppendOutcome {
    Appended {
        sequence: i64,
    },
    /// Same source id, same normalized content: the producer retried and the journal already holds
    /// the assertion, so nothing is inserted, projected, or published a second time.
    IdenticalDuplicate {
        sequence: i64,
    },
    /// Same source id, different content. The original row wins; the caller marks coverage partial.
    Conflict,
}

/// Persistence for the evidence journal and its projections.
///
/// Deliberately narrow. The application never issues SQL, never sees a row type, and never learns
/// how a cursor is encoded — those live behind this port so the use cases can be tested without
/// SQLite and so a storage change cannot leak into policy.
pub(crate) trait EvidenceRepositoryPort: Send + Sync {
    /// Appends one event and updates its projection and coverage metadata in a single
    /// transaction. Returns after commit; publishing is the caller's job precisely so a failed
    /// notice cannot roll back a committed event.
    fn append(
        &self,
        event: &ExecutionEvidenceEvent,
        fingerprint: &str,
        recorded_at: &str,
    ) -> Result<EvidenceAppendOutcome, EvidenceApplicationError>;

    fn list_records(
        &self,
        query: &ExecutionRecordQuery,
    ) -> Result<EvidenceRecordPage, EvidenceApplicationError>;

    fn record_detail(
        &self,
        query: &ExecutionRecordDetailQuery,
    ) -> Result<ExecutionRecordDetailView, EvidenceApplicationError>;

    fn summary(
        &self,
        query: &WorkspaceEvidenceSummaryQuery,
    ) -> Result<WorkspaceEvidenceSummary, EvidenceApplicationError>;

    fn correlation_counts(
        &self,
        session_id: &EvidenceSessionId,
        run_id: Option<&str>,
    ) -> Result<EvidenceCorrelationCounts, EvidenceApplicationError>;

    fn subscription_bootstrap(
        &self,
        session_id: &EvidenceSessionId,
    ) -> Result<EvidenceSubscriptionBootstrap, EvidenceApplicationError>;

    /// Whether the projection still agrees with the journal.
    ///
    /// Two indexed aggregates, not a scan: the newest lifecycle event's sequence against the
    /// newest sequence any projection row has applied. A projection that is missing, empty, or
    /// behind reads as stale; one that is current reads as current without the journal being
    /// touched beyond its index.
    fn projection_is_stale(&self) -> Result<bool, EvidenceApplicationError>;

    /// Rebuilds projections from the retained journal. Appends nothing and publishes nothing, so
    /// it is safe to run at startup: the journal is the record of what happened and the projection
    /// is a cache of it, which is what makes rebuilding cheaper than repairing.
    fn replay_projections(
        &self,
        session_id: Option<&EvidenceSessionId>,
    ) -> Result<usize, EvidenceApplicationError>;

    /// Removes evidence older than the cutoff in bounded batches, returning how much went. The
    /// caller repeats until a pass clears less than a full batch.
    fn maintain_retention(
        &self,
        cutoff: &str,
        now: &str,
    ) -> Result<EvidenceRetentionSummary, EvidenceApplicationError>;
}

/// What one retention pass removed. The caller compares it against the batch size to decide
/// whether a backlog remains, so the two counts are the whole contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct EvidenceRetentionSummary {
    pub(crate) deleted_events: usize,
    pub(crate) deleted_records: usize,
}

pub(crate) trait EvidenceClockPort: Send + Sync {
    fn now_rfc3339(&self) -> String;
}

/// Event ids are generated rather than supplied, so a producer cannot choose one that collides
/// with another producer's. Idempotency rides on `(source_context, source_event_id)` instead.
pub(crate) trait EvidenceIdGeneratorPort: Send + Sync {
    fn next_event_id(&self) -> String;
}

/// A second pair of eyes on an already-validated event.
///
/// The payload enum is the boundary that makes unsafe content unrepresentable; this port exists
/// for the deployment-specific rules that cannot be expressed in a type — an installation's own
/// redaction policy, for instance. It can only reject, never rewrite, so it can never turn an
/// invalid event into a plausible one.
pub(crate) trait EvidenceRedactionValidatorPort: Send + Sync {
    fn validate(&self, event: &ExecutionEvidenceEvent) -> Result<(), EvidenceApplicationError>;
}

/// Publishes after commit. A failure here is reported and dropped: the event is already durable,
/// and rolling it back to keep a notification honest would lose the record entirely.
pub(crate) trait PostCommitEvidenceNoticePublisherPort: Send + Sync {
    fn publish(&self, notice: &EvidenceNotice);
}

/// Where conflicts and drops are reported.
///
/// Rate-limited and redacted by the implementation. The diagnostic carries the source identity and
/// a reason code, never the two payloads or a diff between them — a conflict is usually a producer
/// bug, and dumping both versions to a log would put the content the journal refused into a file.
pub(crate) trait EvidenceGapDiagnosticsPort: Send + Sync {
    fn record_conflict(
        &self,
        source_context: EvidenceSourceContext,
        source_event_id: &SourceEventId,
    );
    fn record_dropped(&self, session_id: &EvidenceSessionId, dropped_count: u32);
}
