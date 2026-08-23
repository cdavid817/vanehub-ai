use super::models::{
    EvidenceNotice, EvidenceNoticeKind, EvidenceRecordPage, EvidenceSubscriptionBootstrap,
    ExecutionRecordDetailQuery, ExecutionRecordDetailView, ExecutionRecordKind,
    ExecutionRecordQuery, RecordEvidenceOutcome, WorkspaceEvidenceSummary,
    WorkspaceEvidenceSummaryQuery, DEFAULT_EVIDENCE_PAGE_SIZE, MAX_EVIDENCE_PAGE_SIZE,
};
use super::ports::{
    EvidenceAppendOutcome, EvidenceApplicationError, EvidenceClockPort, EvidenceGapDiagnosticsPort,
    EvidenceIdGeneratorPort, EvidenceRedactionValidatorPort, EvidenceRepositoryPort,
    EvidenceRetentionSummary, PostCommitEvidenceNoticePublisherPort,
};
#[cfg(test)]
use crate::contexts::execution_observability::domain::{reason_codes, EvidenceCoverageState};
use crate::contexts::execution_observability::domain::{
    EvidenceCommandId, EvidenceCorrelation, EvidenceEventId, EvidenceKind, EvidenceOperationId,
    EvidenceSessionId, EvidenceSourceContext, ExecutionEvidenceEvent, ExecutionEvidenceEventInput,
    ExecutionFidelity, ExecutionRunId, ExecutionStatus, RedactionReceipt, SafeEvidencePayload,
    SourceEventId, SpanId, TraceId, EVIDENCE_SCHEMA_VERSION,
};
use std::sync::Arc;

/// What a producer hands over. It is not an `ExecutionEvidenceEvent` yet: the event id and the
/// recording timestamp are the service's to assign, so a producer cannot choose an id that
/// collides or backdate an observation.
pub(crate) struct RecordEvidenceInput {
    pub(crate) source_context: EvidenceSourceContext,
    pub(crate) source_event_id: SourceEventId,
    pub(crate) occurred_at: String,
    pub(crate) correlation: EvidenceCorrelation,
    pub(crate) status: Option<ExecutionStatus>,
    pub(crate) fidelity: ExecutionFidelity,
    pub(crate) payload: SafeEvidencePayload,
    pub(crate) redaction: RedactionReceipt,
}

#[derive(Clone)]
pub(crate) struct ExecutionEvidenceService {
    repository: Arc<dyn EvidenceRepositoryPort>,
    clock: Arc<dyn EvidenceClockPort>,
    ids: Arc<dyn EvidenceIdGeneratorPort>,
    redaction: Arc<dyn EvidenceRedactionValidatorPort>,
    notices: Arc<dyn PostCommitEvidenceNoticePublisherPort>,
    diagnostics: Arc<dyn EvidenceGapDiagnosticsPort>,
}

impl ExecutionEvidenceService {
    pub(crate) fn new(
        repository: Arc<dyn EvidenceRepositoryPort>,
        clock: Arc<dyn EvidenceClockPort>,
        ids: Arc<dyn EvidenceIdGeneratorPort>,
        redaction: Arc<dyn EvidenceRedactionValidatorPort>,
        notices: Arc<dyn PostCommitEvidenceNoticePublisherPort>,
        diagnostics: Arc<dyn EvidenceGapDiagnosticsPort>,
    ) -> Self {
        Self {
            repository,
            clock,
            ids,
            redaction,
            notices,
            diagnostics,
        }
    }

    /// The write path.
    ///
    /// Order is the design: validate, persist in one transaction, and only then publish. A notice
    /// emitted before commit would tell a subscriber to fetch a row that a rollback then removed,
    /// and a notice that fails after commit is a lost notification rather than a lost event.
    pub(crate) fn record(
        &self,
        input: RecordEvidenceInput,
    ) -> Result<RecordEvidenceOutcome, EvidenceApplicationError> {
        let event = ExecutionEvidenceEvent::new(ExecutionEvidenceEventInput {
            event_id: EvidenceEventId::parse(self.ids.next_event_id())?,
            source_context: input.source_context,
            source_event_id: input.source_event_id,
            schema_version: EVIDENCE_SCHEMA_VERSION,
            occurred_at: input.occurred_at,
            correlation: input.correlation,
            status: input.status,
            fidelity: input.fidelity,
            payload: input.payload,
            redaction: input.redaction,
        })?;
        self.redaction.validate(&event)?;

        let fingerprint = event.canonical_fingerprint();
        let recorded_at = self.clock.now_rfc3339();
        match self.repository.append(&event, &fingerprint, &recorded_at)? {
            EvidenceAppendOutcome::Appended { sequence } => {
                if let Some(notice) = notice_for(&event, sequence) {
                    self.notices.publish(&notice);
                }
                Ok(RecordEvidenceOutcome::Recorded { sequence })
            }
            // A retry must not publish again: a subscriber would count the same record twice.
            EvidenceAppendOutcome::IdenticalDuplicate { sequence } => {
                Ok(RecordEvidenceOutcome::Duplicate { sequence })
            }
            EvidenceAppendOutcome::Conflict => {
                self.diagnostics
                    .record_conflict(event.source_context(), event.source_event_id());
                Ok(RecordEvidenceOutcome::Conflict)
            }
        }
    }

    pub(crate) fn list_records(
        &self,
        mut query: ExecutionRecordQuery,
    ) -> Result<EvidenceRecordPage, EvidenceApplicationError> {
        query.limit = bounded_page_size(query.limit);
        self.repository.list_records(&query)
    }

    pub(crate) fn record_detail(
        &self,
        query: ExecutionRecordDetailQuery,
    ) -> Result<ExecutionRecordDetailView, EvidenceApplicationError> {
        self.repository.record_detail(&query)
    }

    pub(crate) fn summary(
        &self,
        query: WorkspaceEvidenceSummaryQuery,
    ) -> Result<WorkspaceEvidenceSummary, EvidenceApplicationError> {
        self.repository.summary(&query)
    }

    pub(crate) fn subscription_bootstrap(
        &self,
        session_id: &EvidenceSessionId,
    ) -> Result<EvidenceSubscriptionBootstrap, EvidenceApplicationError> {
        self.repository.subscription_bootstrap(session_id)
    }

    /// Reports a producer-side drop so coverage can stop claiming completeness. The count comes
    /// from the producer's bounded queue; the diagnostic carries nothing else.
    pub(crate) fn record_dropped_events(&self, session_id: &EvidenceSessionId, dropped: u32) {
        if dropped == 0 {
            return;
        }
        self.diagnostics.record_dropped(session_id, dropped);
        self.notices.publish(&EvidenceNotice {
            kind: EvidenceNoticeKind::CoverageGap,
            sequence: 0,
            session_id: session_id.clone(),
            occurred_at: self.clock.now_rfc3339(),
            record_id: None,
            run_id: None,
            trace_id: None,
            span_id: None,
            operation_id: None,
            command_id: None,
            seat_id: None,
            dropped_count: Some(dropped),
        });
    }
}

/// What one startup repair did. `AlreadyCurrent` is the normal outcome and the one that must not
/// touch the journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectionRepair {
    AlreadyCurrent,
    Rebuilt { records: usize },
}

/// Maintenance the runtime runs on a schedule rather than on a request path.
///
/// Both live here rather than on the repository's inherent surface so a caller cannot reach past
/// the port to a SQL statement, and so the batching contract — repeat until a pass clears less
/// than a full batch — is stated once instead of at every call site.
impl ExecutionEvidenceService {
    /// Rebuilds the projection only when it no longer agrees with the journal.
    ///
    /// An unconditional rebuild produces the same rows it deleted and pays a full journal scan to
    /// do it, on every launch. Worse, a rebuild that exists to give the replay code a caller is
    /// indistinguishable from one that repairs something — so the check comes first and the
    /// outcome says which of the two happened.
    pub(crate) fn repair_projections_if_needed(
        &self,
    ) -> Result<ProjectionRepair, EvidenceApplicationError> {
        if !self.repository.projection_is_stale()? {
            return Ok(ProjectionRepair::AlreadyCurrent);
        }
        // One transaction covers the delete and every re-insert, so an interrupted rebuild rolls
        // back to the state that was already there rather than leaving a half-built projection.
        // The next startup sees the same staleness and retries; there is no partial state to
        // represent, which is why no in-progress flag is stored.
        let records = self.repository.replay_projections(None)?;
        Ok(ProjectionRepair::Rebuilt { records })
    }

    pub(crate) fn maintain_retention(
        &self,
        cutoff: &str,
    ) -> Result<EvidenceRetentionSummary, EvidenceApplicationError> {
        self.repository
            .maintain_retention(cutoff, &self.clock.now_rfc3339())
    }
}

impl ExecutionEvidenceService {
    /// Records a loss the caller could not attribute. Nothing is persisted: the count degrades
    /// every session's coverage, which is what a reader needs and all this can honestly offer.
    pub(crate) fn report_unattributed_gap(&self, count: u32) {
        self.repository.report_unattributed_gap(count);
    }
}

pub(crate) fn bounded_page_size(limit: usize) -> usize {
    if limit == 0 {
        return DEFAULT_EVIDENCE_PAGE_SIZE;
    }
    limit.min(MAX_EVIDENCE_PAGE_SIZE)
}

/// Builds the identifier-only notice for a committed event.
///
/// Every field here is an id, a sequence, or a classification. The payload is not consulted at
/// all, which is what makes it impossible for command text or a file name to reach the event
/// channel by accident.
fn notice_for(event: &ExecutionEvidenceEvent, sequence: i64) -> Option<EvidenceNotice> {
    let correlation = event.correlation();
    // A completion lands on a record a start already created, so it updates rather than appends.
    // A subscriber that treated both as appends would insert a duplicate row and then have to
    // reconcile it against the page it already has.
    let kind = match ExecutionRecordKind::for_kind(event.kind()) {
        Some(_) if is_lifecycle_completion(event.kind()) => EvidenceNoticeKind::RecordUpdated,
        Some(_) => EvidenceNoticeKind::RecordAppended,
        None => EvidenceNoticeKind::SummaryChanged,
    };
    // A session is a domain invariant, so this is unreachable for a constructed event. Skipping
    // is still the right failure: a notice routed to a placeholder session would be delivered to
    // the wrong subscriber, which is worse than not delivering it at all.
    let session_id = correlation.session().cloned()?;
    Some(EvidenceNotice {
        kind,
        sequence,
        session_id,
        occurred_at: event.occurred_at().to_string(),
        record_id: None,
        run_id: correlation
            .run_id
            .as_ref()
            .map(ExecutionRunId::as_str)
            .map(str::to_string),
        trace_id: correlation
            .trace_id
            .as_ref()
            .map(TraceId::as_str)
            .map(str::to_string),
        span_id: correlation
            .span_id
            .as_ref()
            .map(SpanId::as_str)
            .map(str::to_string),
        operation_id: correlation
            .operation_id
            .as_ref()
            .map(EvidenceOperationId::as_str)
            .map(str::to_string),
        command_id: correlation
            .command_id
            .as_ref()
            .map(EvidenceCommandId::as_str)
            .map(str::to_string),
        seat_id: correlation.seat_id.clone(),
        dropped_count: None,
    })
}

fn is_lifecycle_completion(kind: EvidenceKind) -> bool {
    matches!(
        kind,
        EvidenceKind::CommandCompleted
            | EvidenceKind::ToolCompleted
            | EvidenceKind::AgentCompleted
            | EvidenceKind::VerificationCompleted
    )
}

/// The coverage a store reports before any producer is connected.
///
/// Reached only by tests now that producers publish; the repository builds this state from the
/// store's own row counts rather than from a constant.
#[cfg(test)]
pub(crate) fn capture_not_initialized_coverage(
) -> crate::contexts::execution_observability::domain::QueryCoverage {
    crate::contexts::execution_observability::domain::QueryCoverage::complete().degrade_to(
        EvidenceCoverageState::Partial,
        reason_codes::CAPTURE_NOT_INITIALIZED,
    )
}
