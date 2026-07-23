use super::ExecutionTelemetryError;
use crate::contexts::execution_observability::domain::{
    CapturePolicy, ExecutionContext, ExecutionEvent, ExecutionRun, ExecutionRunId, ExecutionSpan,
    ExecutionStatus, ExecutionTimeline, ObservabilitySettings, Page, PageRequest, SpanId,
};
use std::time::Duration;

pub(crate) trait ExecutionIdentityPort: Send + Sync {
    fn next_context(
        &self,
        capture_policy: CapturePolicy,
        sampling_ratio: f64,
        mcp_relay_enabled: bool,
    ) -> ExecutionContext;
    fn next_span_id(&self) -> SpanId;
}

pub(crate) trait ExecutionSettingsPort: Send + Sync {
    fn load_settings(&self) -> Result<ObservabilitySettings, ExecutionTelemetryError>;
}

pub(crate) trait ObservabilityCredentialPort: Send + Sync {
    fn load_otlp_auth(&self)
        -> Result<Option<zeroize::Zeroizing<String>>, ExecutionTelemetryError>;
    fn set_otlp_auth(&self, secret: &str) -> Result<(), ExecutionTelemetryError>;
    fn delete_otlp_auth(&self) -> Result<(), ExecutionTelemetryError>;
    fn has_otlp_auth(&self) -> Result<bool, ExecutionTelemetryError>;
}

pub(crate) trait ExecutionTelemetryPort: Send + Sync {
    fn start_run(&self, run: &ExecutionRun) -> Result<(), ExecutionTelemetryError>;

    fn finish_run(
        &self,
        run_id: &ExecutionRunId,
        status: ExecutionStatus,
        ended_at: &str,
        error_classification: Option<&str>,
    ) -> Result<(), ExecutionTelemetryError>;

    fn start_span(&self, span: &ExecutionSpan) -> Result<(), ExecutionTelemetryError>;

    fn finish_span(
        &self,
        run_id: &ExecutionRunId,
        span_id: &SpanId,
        status: ExecutionStatus,
        ended_at: &str,
        error_classification: Option<&str>,
    ) -> Result<(), ExecutionTelemetryError>;

    fn record_event(&self, event: &ExecutionEvent) -> Result<(), ExecutionTelemetryError>;

    fn add_metric(
        &self,
        name: &'static str,
        value: u64,
        dimensions: &[(&'static str, &'static str)],
    ) -> Result<(), ExecutionTelemetryError>;

    fn shutdown(&self, _timeout: Duration) -> Result<(), ExecutionTelemetryError> {
        Ok(())
    }
}

pub(crate) trait ExecutionObservabilityRepositoryPort: Send + Sync {
    fn load_settings(&self) -> Result<ObservabilitySettings, ExecutionTelemetryError>;

    fn update_settings(
        &self,
        settings: &ObservabilitySettings,
        updated_at: &str,
    ) -> Result<(), ExecutionTelemetryError>;

    fn list_runs(
        &self,
        request: &PageRequest,
        session_id: Option<&str>,
    ) -> Result<Page<ExecutionRun>, ExecutionTelemetryError>;

    fn timeline(
        &self,
        run_id: &ExecutionRunId,
    ) -> Result<Option<ExecutionTimeline>, ExecutionTelemetryError>;
}
