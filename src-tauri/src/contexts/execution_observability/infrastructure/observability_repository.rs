use super::SqliteExecutionTimelineRepository;
use crate::contexts::execution_observability::application::{
    ExecutionObservabilityRepositoryPort, ExecutionTelemetryError,
};
use crate::contexts::execution_observability::domain::{
    ExecutionRun, ExecutionRunId, ExecutionTimeline, ObservabilitySettings, Page, PageRequest,
};

impl ExecutionObservabilityRepositoryPort for SqliteExecutionTimelineRepository {
    fn load_settings(&self) -> Result<ObservabilitySettings, ExecutionTelemetryError> {
        SqliteExecutionTimelineRepository::load_settings(self)
    }

    fn update_settings(
        &self,
        settings: &ObservabilitySettings,
        updated_at: &str,
    ) -> Result<(), ExecutionTelemetryError> {
        SqliteExecutionTimelineRepository::update_settings(self, settings, updated_at)
    }

    fn list_runs(
        &self,
        request: &PageRequest,
        session_id: Option<&str>,
    ) -> Result<Page<ExecutionRun>, ExecutionTelemetryError> {
        SqliteExecutionTimelineRepository::list_runs(self, request, session_id)
    }

    fn timeline(
        &self,
        run_id: &ExecutionRunId,
    ) -> Result<Option<ExecutionTimeline>, ExecutionTelemetryError> {
        SqliteExecutionTimelineRepository::timeline(self, run_id)
    }

    fn timeline_page(
        &self,
        run_id: &ExecutionRunId,
        event_limit: usize,
        event_page_token: Option<&str>,
    ) -> Result<Option<ExecutionTimeline>, ExecutionTelemetryError> {
        SqliteExecutionTimelineRepository::timeline_page(
            self,
            run_id,
            event_limit,
            event_page_token,
        )
    }
}
