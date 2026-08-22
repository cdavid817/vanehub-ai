use super::application::evidence::{
    EvidenceApplicationError, EvidenceClockPort, EvidenceCorrelationCounts,
    EvidenceGapDiagnosticsPort, EvidenceIdGeneratorPort, EvidenceRecordPage,
    EvidenceRedactionValidatorPort, EvidenceRepositoryPort, EvidenceSubscriptionBootstrap,
    ExecutionEvidenceService, ExecutionRecordDetailQuery, ExecutionRecordDetailView,
    ExecutionRecordQuery, PostCommitEvidenceNoticePublisherPort, RecordEvidenceInput,
    RecordEvidenceOutcome, WorkspaceEvidenceSummary, WorkspaceEvidenceSummaryQuery,
};
use super::domain::EvidenceSessionId;
use std::sync::Arc;

/// The published evidence contract.
///
/// Everything the rest of the application may touch passes through here. The repository, the row
/// types, the SQLite schema, the migration, and the cursor encoding stay private to the context:
/// a consumer that could reach them would be able to write a query the coverage rules do not
/// know about, and a page whose completeness nobody can vouch for is exactly what this capability
/// exists to eliminate.
///
/// The recorder is intentionally not reachable from a Tauri command. Evidence is written by
/// producers inside the process, never by the frontend, so no client can inject an observation.
#[derive(Clone)]
pub(crate) struct ExecutionEvidenceApi {
    service: ExecutionEvidenceService,
}

impl ExecutionEvidenceApi {
    pub(crate) fn new(
        repository: Arc<dyn EvidenceRepositoryPort>,
        clock: Arc<dyn EvidenceClockPort>,
        ids: Arc<dyn EvidenceIdGeneratorPort>,
        redaction: Arc<dyn EvidenceRedactionValidatorPort>,
        notices: Arc<dyn PostCommitEvidenceNoticePublisherPort>,
        diagnostics: Arc<dyn EvidenceGapDiagnosticsPort>,
    ) -> Self {
        Self {
            service: ExecutionEvidenceService::new(
                repository,
                clock,
                ids,
                redaction,
                notices,
                diagnostics,
            ),
        }
    }

    /// In-process producers only. Task Group 4 supplies the bootstrap adapters that call this.
    pub(crate) fn record(
        &self,
        input: RecordEvidenceInput,
    ) -> Result<RecordEvidenceOutcome, EvidenceApplicationError> {
        self.service.record(input)
    }

    pub(crate) fn record_dropped_events(&self, session_id: &EvidenceSessionId, dropped: u32) {
        self.service.record_dropped_events(session_id, dropped);
    }

    pub(crate) fn summary(
        &self,
        query: WorkspaceEvidenceSummaryQuery,
    ) -> Result<WorkspaceEvidenceSummary, EvidenceApplicationError> {
        self.service.summary(query)
    }

    pub(crate) fn list_records(
        &self,
        query: ExecutionRecordQuery,
    ) -> Result<EvidenceRecordPage, EvidenceApplicationError> {
        self.service.list_records(query)
    }

    pub(crate) fn record_detail(
        &self,
        query: ExecutionRecordDetailQuery,
    ) -> Result<ExecutionRecordDetailView, EvidenceApplicationError> {
        self.service.record_detail(query)
    }

    pub(crate) fn correlation_counts(
        &self,
        session_id: &EvidenceSessionId,
        run_id: Option<&str>,
    ) -> Result<EvidenceCorrelationCounts, EvidenceApplicationError> {
        self.service.correlation_counts(session_id, run_id)
    }

    pub(crate) fn subscription_bootstrap(
        &self,
        session_id: &EvidenceSessionId,
    ) -> Result<EvidenceSubscriptionBootstrap, EvidenceApplicationError> {
        self.service.subscription_bootstrap(session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::execution_observability::application::evidence::models::{
        EvidenceQueryScope, ExecutionRecordFilters,
    };
    use crate::contexts::execution_observability::application::evidence::ports::EvidenceAppendOutcome;
    use crate::contexts::execution_observability::domain::evidence::builders::CoverageBuilder;
    use crate::contexts::execution_observability::domain::{
        EvidenceSourceContext, ExecutionEvidenceEvent, QueryCoverage, SourceEventId,
    };
    use crate::contexts::execution_observability::ExecutionEvidenceApi as PublishedApi;
    use std::sync::Mutex;

    #[derive(Default)]
    struct StubRepository {
        calls: Mutex<Vec<&'static str>>,
    }

    impl EvidenceRepositoryPort for StubRepository {
        fn append(
            &self,
            _event: &ExecutionEvidenceEvent,
            _fingerprint: &str,
            _recorded_at: &str,
        ) -> Result<EvidenceAppendOutcome, EvidenceApplicationError> {
            self.calls.lock().expect("calls").push("append");
            Ok(EvidenceAppendOutcome::Appended { sequence: 1 })
        }

        fn list_records(
            &self,
            _query: &ExecutionRecordQuery,
        ) -> Result<EvidenceRecordPage, EvidenceApplicationError> {
            self.calls.lock().expect("calls").push("list");
            Ok(EvidenceRecordPage {
                items: Vec::new(),
                next_cursor: None,
                coverage: CoverageBuilder::capture_not_initialized(),
            })
        }

        fn record_detail(
            &self,
            _query: &ExecutionRecordDetailQuery,
        ) -> Result<ExecutionRecordDetailView, EvidenceApplicationError> {
            self.calls.lock().expect("calls").push("detail");
            Err(EvidenceApplicationError::RecordNotFound)
        }

        fn summary(
            &self,
            query: &WorkspaceEvidenceSummaryQuery,
        ) -> Result<WorkspaceEvidenceSummary, EvidenceApplicationError> {
            self.calls.lock().expect("calls").push("summary");
            Ok(WorkspaceEvidenceSummary {
                session_id: query.session_id.clone(),
                generated_at: "2026-01-01T00:00:00Z".to_string(),
                coverage: CoverageBuilder::capture_not_initialized(),
                run_status: None,
                run_id: None,
                run_started_at: None,
                running_records: 0,
                failed_records: 0,
                verification_passed: 0,
                verification_failed: 0,
                unowned_sources: Vec::new(),
            })
        }

        fn correlation_counts(
            &self,
            _session_id: &EvidenceSessionId,
            _run_id: Option<&str>,
        ) -> Result<EvidenceCorrelationCounts, EvidenceApplicationError> {
            self.calls.lock().expect("calls").push("counts");
            Ok(EvidenceCorrelationCounts::default())
        }

        fn subscription_bootstrap(
            &self,
            session_id: &EvidenceSessionId,
        ) -> Result<EvidenceSubscriptionBootstrap, EvidenceApplicationError> {
            self.calls.lock().expect("calls").push("bootstrap");
            Ok(EvidenceSubscriptionBootstrap {
                session_id: session_id.clone(),
                watermark_sequence: 0,
                coverage: QueryCoverage::complete(),
            })
        }
    }

    struct StubClock;
    impl EvidenceClockPort for StubClock {
        fn now_rfc3339(&self) -> String {
            "2026-01-01T00:00:00.000Z".to_string()
        }
    }

    struct StubIds;
    impl EvidenceIdGeneratorPort for StubIds {
        fn next_event_id(&self) -> String {
            "event-1".to_string()
        }
    }

    struct StubValidator;
    impl EvidenceRedactionValidatorPort for StubValidator {
        fn validate(
            &self,
            _event: &ExecutionEvidenceEvent,
        ) -> Result<(), EvidenceApplicationError> {
            Ok(())
        }
    }

    struct StubNotices;
    impl PostCommitEvidenceNoticePublisherPort for StubNotices {
        fn publish(
            &self,
            _notice: &crate::contexts::execution_observability::application::evidence::models::EvidenceNotice,
        ) {
        }
    }

    struct StubDiagnostics;
    impl EvidenceGapDiagnosticsPort for StubDiagnostics {
        fn record_conflict(&self, _context: EvidenceSourceContext, _id: &SourceEventId) {}
        fn record_dropped(&self, _session_id: &EvidenceSessionId, _dropped: u32) {}
    }

    fn api(repository: Arc<StubRepository>) -> PublishedApi {
        PublishedApi::new(
            repository,
            Arc::new(StubClock),
            Arc::new(StubIds),
            Arc::new(StubValidator),
            Arc::new(StubNotices),
            Arc::new(StubDiagnostics),
        )
    }

    /// The published surface is exactly the four reads plus the in-process recorder. If a query
    /// stopped reaching the service this would keep passing only if the delegation were removed
    /// entirely, which is the mistake worth catching.
    #[test]
    fn every_published_query_reaches_the_service() {
        let repository = Arc::new(StubRepository::default());
        let api = api(repository.clone());
        let session = EvidenceSessionId::parse("session-1").expect("session");

        api.summary(WorkspaceEvidenceSummaryQuery {
            session_id: session.clone(),
            seat_id: None,
        })
        .expect("summary");
        api.list_records(ExecutionRecordQuery {
            scope: EvidenceQueryScope {
                session_id: Some(session.clone()),
                ..EvidenceQueryScope::default()
            },
            filters: ExecutionRecordFilters::default(),
            cursor: None,
            limit: 10,
        })
        .expect("records");
        api.record_detail(ExecutionRecordDetailQuery {
            session_id: session.clone(),
            record_id: "record-1".to_string(),
        })
        .expect_err("record is absent in the stub");
        api.correlation_counts(&session, None).expect("counts");
        api.subscription_bootstrap(&session).expect("bootstrap");

        assert_eq!(
            *repository.calls.lock().expect("calls"),
            vec!["summary", "list", "detail", "counts", "bootstrap"]
        );
    }
}
