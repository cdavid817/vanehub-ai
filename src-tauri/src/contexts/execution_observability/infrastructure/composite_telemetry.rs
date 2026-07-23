use crate::contexts::execution_observability::application::{
    ExecutionTelemetryError, ExecutionTelemetryPort,
};
use crate::contexts::execution_observability::domain::{
    ExecutionEvent, ExecutionRun, ExecutionRunId, ExecutionSpan, ExecutionStatus, SpanId,
};
use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::privacy::sanitize_attributes;

const EXPORT_FAILURE_DIAGNOSTIC_INTERVAL: Duration = Duration::from_secs(60);

#[cfg(test)]
#[path = "composite_test_support.rs"]
mod composite_test_support;

#[derive(Clone)]
pub(crate) struct CompositeExecutionTelemetry {
    local: Arc<dyn ExecutionTelemetryPort>,
    exporters: Vec<Arc<dyn ExecutionTelemetryPort>>,
    dropped_exports: Arc<AtomicU64>,
    diagnostics: Option<Arc<dyn DiagnosticLogPort>>,
    last_export_diagnostic: Arc<Mutex<Option<Instant>>>,
    capture_policies: Arc<
        Mutex<HashMap<String, crate::contexts::execution_observability::domain::CapturePolicy>>,
    >,
}

impl CompositeExecutionTelemetry {
    pub(crate) fn new(
        local: Arc<dyn ExecutionTelemetryPort>,
        exporters: Vec<Arc<dyn ExecutionTelemetryPort>>,
    ) -> Self {
        Self {
            local,
            exporters,
            dropped_exports: Arc::new(AtomicU64::new(0)),
            diagnostics: None,
            last_export_diagnostic: Arc::new(Mutex::new(None)),
            capture_policies: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) fn with_diagnostics(
        local: Arc<dyn ExecutionTelemetryPort>,
        exporters: Vec<Arc<dyn ExecutionTelemetryPort>>,
        diagnostics: Arc<dyn DiagnosticLogPort>,
    ) -> Self {
        let mut composite = Self::new(local, exporters);
        composite.diagnostics = Some(diagnostics);
        composite
    }

    #[cfg(test)]
    pub(crate) fn dropped_exports(&self) -> u64 {
        self.dropped_exports.load(Ordering::Relaxed)
    }

    fn export(
        &self,
        operation: impl Fn(&dyn ExecutionTelemetryPort) -> Result<(), ExecutionTelemetryError>,
    ) {
        for exporter in &self.exporters {
            if operation(exporter.as_ref()).is_err() {
                self.dropped_exports.fetch_add(1, Ordering::Relaxed);
                let _ = self.local.add_metric(
                    "vanehub.telemetry.export.dropped",
                    1,
                    &[("signal", "execution")],
                );
                self.record_export_failure();
            }
        }
    }

    fn record_export_failure(&self) {
        let Some(diagnostics) = &self.diagnostics else {
            return;
        };
        let Ok(mut last) = self.last_export_diagnostic.lock() else {
            return;
        };
        let now = Instant::now();
        if last.is_some_and(|previous| {
            now.duration_since(previous) < EXPORT_FAILURE_DIAGNOSTIC_INTERVAL
        }) {
            return;
        }
        *last = Some(now);
        drop(last);
        let _ = diagnostics.write_diagnostic(DiagnosticLog {
            severity: LogSeverity::Warn,
            category: "execution_observability.export".to_string(),
            message: "Execution telemetry export is degraded; local task execution remains active"
                .to_string(),
            context: BTreeMap::from([
                ("signal".to_string(), "execution".to_string()),
                ("action".to_string(), "rate_limited_diagnostic".to_string()),
            ]),
        });
    }
}

impl ExecutionTelemetryPort for CompositeExecutionTelemetry {
    fn start_run(&self, run: &ExecutionRun) -> Result<(), ExecutionTelemetryError> {
        let mut sanitized = run.clone();
        sanitized.attributes = sanitize_attributes(run.context.capture_policy, &run.attributes);
        if let Ok(mut policies) = self.capture_policies.lock() {
            policies.insert(
                run.context.run_id.as_str().to_string(),
                run.context.capture_policy,
            );
        }
        self.local.start_run(&sanitized)?;
        self.export(|exporter| exporter.start_run(&sanitized));
        self.add_metric(
            "vanehub.execution.run.started",
            1,
            &[("source", source_dimension(&run.source))],
        )?;
        Ok(())
    }

    fn finish_run(
        &self,
        run_id: &ExecutionRunId,
        status: ExecutionStatus,
        ended_at: &str,
        error_classification: Option<&str>,
    ) -> Result<(), ExecutionTelemetryError> {
        self.local
            .finish_run(run_id, status, ended_at, error_classification)?;
        self.export(|exporter| exporter.finish_run(run_id, status, ended_at, error_classification));
        if let Ok(mut policies) = self.capture_policies.lock() {
            policies.remove(run_id.as_str());
        }
        self.add_metric(
            "vanehub.execution.run.completed",
            1,
            &[("outcome", status_dimension(status))],
        )?;
        Ok(())
    }

    fn start_span(&self, span: &ExecutionSpan) -> Result<(), ExecutionTelemetryError> {
        let mut sanitized = span.clone();
        sanitized.attributes = sanitize_attributes(span.context.capture_policy, &span.attributes);
        self.local.start_span(&sanitized)?;
        self.export(|exporter| exporter.start_span(&sanitized));
        Ok(())
    }

    fn finish_span(
        &self,
        run_id: &ExecutionRunId,
        span_id: &SpanId,
        status: ExecutionStatus,
        ended_at: &str,
        error_classification: Option<&str>,
    ) -> Result<(), ExecutionTelemetryError> {
        self.local
            .finish_span(run_id, span_id, status, ended_at, error_classification)?;
        self.export(|exporter| {
            exporter.finish_span(run_id, span_id, status, ended_at, error_classification)
        });
        Ok(())
    }

    fn record_event(&self, event: &ExecutionEvent) -> Result<(), ExecutionTelemetryError> {
        let mut sanitized = event.clone();
        let policy = self
            .capture_policies
            .lock()
            .ok()
            .and_then(|policies| policies.get(event.run_id.as_str()).copied())
            .unwrap_or(
                crate::contexts::execution_observability::domain::CapturePolicy::MetadataOnly,
            );
        sanitized.attributes = sanitize_attributes(policy, &event.attributes);
        self.local.record_event(&sanitized)?;
        self.export(|exporter| exporter.record_event(&sanitized));
        Ok(())
    }

    fn add_metric(
        &self,
        name: &'static str,
        value: u64,
        dimensions: &[(&'static str, &'static str)],
    ) -> Result<(), ExecutionTelemetryError> {
        self.local.add_metric(name, value, dimensions)?;
        self.export(|exporter| exporter.add_metric(name, value, dimensions));
        Ok(())
    }

    fn shutdown(&self, timeout: Duration) -> Result<(), ExecutionTelemetryError> {
        for exporter in &self.exporters {
            if exporter.shutdown(timeout).is_err() {
                self.dropped_exports.fetch_add(1, Ordering::Relaxed);
                self.record_export_failure();
            }
        }
        self.local.shutdown(timeout)
    }
}

fn source_dimension(
    source: &crate::contexts::execution_observability::domain::ExecutionSource,
) -> &'static str {
    match source {
        crate::contexts::execution_observability::domain::ExecutionSource::Desktop => "desktop",
        crate::contexts::execution_observability::domain::ExecutionSource::InstantMessage {
            ..
        } => "instant_message",
        crate::contexts::execution_observability::domain::ExecutionSource::Scheduled { .. } => {
            "scheduled"
        }
    }
}

fn status_dimension(status: ExecutionStatus) -> &'static str {
    match status {
        ExecutionStatus::Accepted => "accepted",
        ExecutionStatus::Running => "running",
        ExecutionStatus::Succeeded => "succeeded",
        ExecutionStatus::Failed => "failed",
        ExecutionStatus::Cancelled => "cancelled",
        ExecutionStatus::Incomplete => "incomplete",
    }
}

#[cfg(test)]
mod tests {
    use super::composite_test_support::CapturingDiagnostics;
    use super::*;
    use crate::contexts::execution_observability::application::test_adapter::{
        CapturedTelemetryRecord, CapturingExecutionTelemetry,
    };
    use crate::contexts::execution_observability::domain::{
        CapturePolicy, ExecutionContext, ExecutionSource, SafeAttributeValue, SafeAttributes,
        TraceId,
    };
    use std::thread;

    #[derive(Clone, Default)]
    struct FailingTelemetry {
        calls: Arc<AtomicU64>,
    }

    impl FailingTelemetry {
        fn fail<T>(&self) -> Result<T, ExecutionTelemetryError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            failure()
        }
    }

    impl ExecutionTelemetryPort for FailingTelemetry {
        fn start_run(&self, _run: &ExecutionRun) -> Result<(), ExecutionTelemetryError> {
            self.fail()
        }

        fn finish_run(
            &self,
            _run_id: &ExecutionRunId,
            _status: ExecutionStatus,
            _ended_at: &str,
            _error_classification: Option<&str>,
        ) -> Result<(), ExecutionTelemetryError> {
            self.fail()
        }

        fn start_span(&self, _span: &ExecutionSpan) -> Result<(), ExecutionTelemetryError> {
            self.fail()
        }

        fn finish_span(
            &self,
            _run_id: &ExecutionRunId,
            _span_id: &SpanId,
            _status: ExecutionStatus,
            _ended_at: &str,
            _error_classification: Option<&str>,
        ) -> Result<(), ExecutionTelemetryError> {
            self.fail()
        }

        fn record_event(&self, _event: &ExecutionEvent) -> Result<(), ExecutionTelemetryError> {
            self.fail()
        }

        fn add_metric(
            &self,
            _name: &'static str,
            _value: u64,
            _dimensions: &[(&'static str, &'static str)],
        ) -> Result<(), ExecutionTelemetryError> {
            self.fail()
        }
    }

    #[test]
    fn local_storage_is_authoritative_and_export_is_best_effort() {
        let local = CapturingExecutionTelemetry::default();
        let exported = CapturingExecutionTelemetry::default();
        let composite = CompositeExecutionTelemetry::new(
            Arc::new(local.clone()),
            vec![Arc::new(exported.clone())],
        );

        composite.start_run(&run()).expect("local write");

        assert!(local
            .records()
            .expect("local")
            .iter()
            .any(|record| matches!(record, CapturedTelemetryRecord::RunStarted(_))));
        assert!(exported
            .records()
            .expect("exported")
            .iter()
            .any(|record| matches!(record, CapturedTelemetryRecord::RunStarted(_))));
        assert_eq!(composite.dropped_exports(), 0);
    }

    #[test]
    fn exporter_failure_does_not_fail_the_user_operation() {
        let local = CapturingExecutionTelemetry::default();
        let diagnostics = Arc::new(CapturingDiagnostics::default());
        let failing = FailingTelemetry::default();
        let composite = CompositeExecutionTelemetry::with_diagnostics(
            Arc::new(local.clone()),
            vec![Arc::new(failing.clone())],
            diagnostics.clone(),
        );

        composite.start_run(&run()).expect("local write survives");
        composite
            .start_run(&run())
            .expect("repeated failure survives");

        assert_eq!(composite.dropped_exports(), 4);
        assert_eq!(failing.calls.load(Ordering::Relaxed), 4);
        assert!(local
            .records()
            .expect("local")
            .iter()
            .any(|record| matches!(
                record,
                CapturedTelemetryRecord::Metric {
                    name: "vanehub.telemetry.export.dropped",
                    dimensions,
                    ..
                } if dimensions == &vec![("signal", "execution")]
            )));
        let logs = diagnostics.logs.lock().expect("diagnostic logs");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].category, "execution_observability.export");
        assert!(!logs[0].message.contains("export unavailable"));
    }

    #[test]
    fn concurrent_run_signals_remain_lossless_in_local_storage() {
        let local = CapturingExecutionTelemetry::default();
        let composite = Arc::new(CompositeExecutionTelemetry::new(
            Arc::new(local.clone()),
            Vec::new(),
        ));
        let workers = (0..32)
            .map(|_| {
                let composite = composite.clone();
                thread::spawn(move || composite.start_run(&run()))
            })
            .collect::<Vec<_>>();
        for worker in workers {
            worker.join().expect("worker").expect("local signal");
        }
        let records = local.records().expect("local");
        assert_eq!(
            records
                .iter()
                .filter(|record| matches!(record, CapturedTelemetryRecord::RunStarted(_)))
                .count(),
            32
        );
        assert_eq!(composite.dropped_exports(), 0);
    }

    #[test]
    fn metadata_only_sanitizes_local_and_exported_records_before_fan_out() {
        let local = CapturingExecutionTelemetry::default();
        let exported = CapturingExecutionTelemetry::default();
        let composite = CompositeExecutionTelemetry::new(
            Arc::new(local.clone()),
            vec![Arc::new(exported.clone())],
        );
        let mut sensitive_run = run();
        sensitive_run.attributes = SafeAttributes::try_from_entries([
            (
                "gen_ai.prompt".to_string(),
                SafeAttributeValue::String("private prompt".to_string()),
            ),
            (
                "tool.arguments".to_string(),
                SafeAttributeValue::String("credential=tool-secret".to_string()),
            ),
            (
                "http.request.headers".to_string(),
                SafeAttributeValue::String("Authorization: Bearer header-secret".to_string()),
            ),
            (
                "process.environment".to_string(),
                SafeAttributeValue::String("TOKEN=environment-secret".to_string()),
            ),
            (
                "safe.detail".to_string(),
                SafeAttributeValue::String(
                    "Bearer token-secret C:\\Users\\developer\\private.json".to_string(),
                ),
            ),
        ])
        .expect("attributes");

        composite.start_run(&sensitive_run).expect("fan out");

        for records in [
            local.records().expect("local"),
            exported.records().expect("exported"),
        ] {
            let CapturedTelemetryRecord::RunStarted(run) = &records[0] else {
                panic!("run record expected");
            };
            let rendered = format!("{:?}", run.attributes.entries());
            for secret in [
                "private prompt",
                "tool-secret",
                "header-secret",
                "environment-secret",
                "token-secret",
                "developer",
                "private.json",
            ] {
                assert!(!rendered.contains(secret), "telemetry leaked {secret}");
            }
            assert!(rendered.contains("[REDACTED]"));
            assert!(rendered.contains("[REDACTED_PATH]"));
        }
    }

    fn run() -> ExecutionRun {
        ExecutionRun {
            context: ExecutionContext {
                run_id: ExecutionRunId::parse("018f0f17-4d6a-7e20-b41d-66c5271a28d0")
                    .expect("run id"),
                trace_id: TraceId::parse("4bf92f3577b34da6a3ce929d0e0e4736").expect("trace id"),
                span_id: SpanId::parse("00f067aa0ba902b7").expect("span id"),
                capture_policy: CapturePolicy::MetadataOnly,
                sampling_per_million: 1_000_000,
                mcp_relay_enabled: false,
            },
            source: ExecutionSource::Desktop,
            status: ExecutionStatus::Running,
            started_at: "2026-07-23T00:00:00Z".to_string(),
            ended_at: None,
            error_classification: None,
            session_id: None,
            user_message_id: None,
            assistant_message_id: None,
            operation_id: None,
            agent_id: None,
            provider_session_id: None,
            attributes: SafeAttributes::default(),
            links: Vec::new(),
        }
    }

    fn failure<T>() -> Result<T, ExecutionTelemetryError> {
        Err(ExecutionTelemetryError::Unavailable(
            "export unavailable".to_string(),
        ))
    }
}
