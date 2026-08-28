use crate::contexts::execution_observability::application::{
    ExecutionTelemetryError, ExecutionTelemetryPort, TraceTransitionKind, TraceTransitionNotice,
    TraceTransitionPublisherPort,
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
    /// Run id to trace id, so a finish can announce the same correlation a start did.
    ///
    /// The finish calls carry only a run id. Without this the trace id on a notice would be
    /// present for a start and missing for a finish, and every consumer would have to handle both.
    run_traces: Arc<Mutex<HashMap<String, String>>>,
    transitions: Option<Arc<dyn TraceTransitionPublisherPort>>,
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
            run_traces: Arc::new(Mutex::new(HashMap::new())),
            transitions: None,
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

    /// Attaches somewhere to announce committed transitions.
    ///
    /// Optional, because telemetry is assembled in places that have no window to publish to — and
    /// a composite that required one would make those paths carry a publisher that does nothing.
    pub(crate) fn with_transitions(
        mut self,
        transitions: Arc<dyn TraceTransitionPublisherPort>,
    ) -> Self {
        self.transitions = Some(transitions);
        self
    }

    /// Announces a transition the local store has already committed.
    ///
    /// After the commit and never before: a notice that arrived first would send a subscriber to
    /// fetch a timeline that does not yet hold the change, and the refetch would return the old
    /// state — which reads as a wrong notice rather than an early one.
    fn announce(
        &self,
        kind: TraceTransitionKind,
        run_id: &ExecutionRunId,
        span_id: Option<&SpanId>,
        status: ExecutionStatus,
        occurred_at: Option<&str>,
    ) {
        let Some(transitions) = self.transitions.as_ref() else {
            return;
        };
        let trace_id = self
            .run_traces
            .lock()
            .ok()
            .and_then(|traces| traces.get(run_id.as_str()).cloned());
        // A transition for a run this process never started has no correlation to announce.
        // Publishing it with an empty trace id would hand a subscriber a value that looks like an
        // id and matches nothing.
        let Some(trace_id) = trace_id else {
            return;
        };
        transitions.publish(&TraceTransitionNotice {
            kind,
            run_id: run_id.as_str().to_string(),
            trace_id,
            span_id: span_id.map(|value| value.as_str().to_string()),
            status,
            occurred_at: occurred_at.map(str::to_string),
        });
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
        if let Ok(mut traces) = self.run_traces.lock() {
            traces.insert(
                run.context.run_id.as_str().to_string(),
                run.context.trace_id.as_str().to_string(),
            );
        }
        self.local.start_run(&sanitized)?;
        self.announce(
            TraceTransitionKind::RunStarted,
            &run.context.run_id,
            None,
            run.status,
            None,
        );
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
        self.announce(
            TraceTransitionKind::RunFinished,
            run_id,
            None,
            status,
            Some(ended_at),
        );
        self.export(|exporter| exporter.finish_run(run_id, status, ended_at, error_classification));
        if let Ok(mut policies) = self.capture_policies.lock() {
            policies.remove(run_id.as_str());
        }
        // Forgotten only after the announcement: the notice needs the correlation this drops.
        if let Ok(mut traces) = self.run_traces.lock() {
            traces.remove(run_id.as_str());
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
        self.announce(
            TraceTransitionKind::SpanStarted,
            &span.context.run_id,
            Some(&span.context.span_id),
            span.status,
            None,
        );
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
        self.announce(
            TraceTransitionKind::SpanFinished,
            run_id,
            Some(span_id),
            status,
            Some(ended_at),
        );
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
                assert!(
                    !rendered.contains(secret),
                    "telemetry leaked a protected value"
                );
            }
            assert!(rendered.contains("[REDACTED]"));
            assert!(rendered.contains("[REDACTED_PATH]"));
        }
    }

    #[test]
    fn performance_evidence_keeps_safe_correlation_when_export_fails() {
        let local = CapturingExecutionTelemetry::default();
        let diagnostics = Arc::new(CapturingDiagnostics::default());
        let composite = CompositeExecutionTelemetry::with_diagnostics(
            Arc::new(local.clone()),
            vec![Arc::new(FailingTelemetry::default())],
            diagnostics,
        );
        let mut measured_run = run();
        measured_run.operation_id = Some("operation-performance-1".to_string());
        measured_run.attributes = SafeAttributes::try_from_entries([
            (
                "performance.metric.id".to_string(),
                SafeAttributeValue::String("run.transition-p95".to_string()),
            ),
            (
                "performance.dataset.id".to_string(),
                SafeAttributeValue::String("runs-1000".to_string()),
            ),
            (
                "performance.dataset.version".to_string(),
                SafeAttributeValue::Integer(1),
            ),
            (
                "performance.baseline".to_string(),
                SafeAttributeValue::Integer(31_316),
            ),
            (
                "performance.measured".to_string(),
                SafeAttributeValue::Integer(31_316),
            ),
            (
                "performance.budget".to_string(),
                SafeAttributeValue::Integer(39_145),
            ),
            (
                "performance.delta".to_string(),
                SafeAttributeValue::Integer(0),
            ),
            (
                "gen_ai.prompt".to_string(),
                SafeAttributeValue::String("must not survive".to_string()),
            ),
        ])
        .expect("performance attributes");

        composite
            .start_run(&measured_run)
            .expect("export failure must not change the run outcome");

        let records = local.records().expect("local evidence");
        let CapturedTelemetryRecord::RunStarted(recorded) = &records[0] else {
            panic!("run record expected");
        };
        assert_eq!(recorded.context.run_id, measured_run.context.run_id);
        assert_eq!(recorded.operation_id, measured_run.operation_id);
        assert_eq!(
            recorded.attributes.entries().get("performance.dataset.id"),
            Some(&SafeAttributeValue::String("runs-1000".to_string()))
        );
        assert!(!recorded.attributes.entries().contains_key("gen_ai.prompt"));
        assert!(composite.dropped_exports() >= 1);
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

    /// Records what it was told, and what the local store held when it was told.
    ///
    /// The second half is the assertion. "Published after the commit" is an ordering claim, and the
    /// only way to check an ordering from inside a publisher is to look at the world at the moment
    /// it was called.
    #[derive(Default)]
    struct RecordingTransitions {
        notices: Mutex<Vec<(TraceTransitionNotice, usize)>>,
        local: Mutex<Option<Arc<CapturingExecutionTelemetry>>>,
    }

    impl RecordingTransitions {
        fn watching(local: Arc<CapturingExecutionTelemetry>) -> Arc<Self> {
            let recorder = Arc::new(Self::default());
            *recorder.local.lock().expect("local") = Some(local);
            recorder
        }

        fn taken(&self) -> Vec<(TraceTransitionNotice, usize)> {
            self.notices.lock().expect("notices").clone()
        }
    }

    impl TraceTransitionPublisherPort for RecordingTransitions {
        fn publish(&self, notice: &TraceTransitionNotice) {
            let committed = self
                .local
                .lock()
                .expect("local")
                .as_ref()
                .map(|local| {
                    local
                        .records()
                        .map(|records| records.len())
                        .unwrap_or_default()
                })
                .unwrap_or_default();
            self.notices
                .lock()
                .expect("notices")
                .push((notice.clone(), committed));
        }
    }

    /// Every transition is announced, and never before the local store has it.
    ///
    /// A notice that arrived first would send a subscriber to fetch a timeline that does not yet
    /// contain the change it was told about — and the refetch would return the old state, which
    /// reads as a wrong notice rather than an early one.
    #[test]
    fn transitions_are_announced_only_after_the_local_store_commits() {
        let local = Arc::new(CapturingExecutionTelemetry::default());
        let transitions = RecordingTransitions::watching(local.clone());
        let telemetry = CompositeExecutionTelemetry::new(local.clone(), Vec::new())
            .with_transitions(transitions.clone());
        let run = run();

        telemetry.start_run(&run).expect("start run");
        telemetry
            .finish_run(
                &run.context.run_id,
                ExecutionStatus::Succeeded,
                "2026-07-23T00:00:05Z",
                None,
            )
            .expect("finish run");

        let taken = transitions.taken();
        let kinds: Vec<TraceTransitionKind> = taken.iter().map(|(notice, _)| notice.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TraceTransitionKind::RunStarted,
                TraceTransitionKind::RunFinished
            ]
        );
        // At least one record was already committed when each notice went out.
        for (notice, committed) in &taken {
            assert!(
                *committed > 0,
                "{:?} was announced before anything was committed",
                notice.kind
            );
        }
    }

    /// A finish carries the same correlation its start did.
    ///
    /// The finish call knows only a run id. A notice whose trace id were present for a start and
    /// missing for a finish is one every consumer has to handle twice.
    #[test]
    fn a_finish_announces_the_same_trace_a_start_did() {
        let local = Arc::new(CapturingExecutionTelemetry::default());
        let transitions = RecordingTransitions::watching(local.clone());
        let telemetry = CompositeExecutionTelemetry::new(local, Vec::new())
            .with_transitions(transitions.clone());
        let run = run();

        telemetry.start_run(&run).expect("start run");
        telemetry
            .finish_run(
                &run.context.run_id,
                ExecutionStatus::Succeeded,
                "2026-07-23T00:00:05Z",
                None,
            )
            .expect("finish run");

        let taken = transitions.taken();
        assert_eq!(taken.len(), 2);
        assert!(taken
            .iter()
            .all(|(notice, _)| notice.trace_id == run.context.trace_id.as_str()));
        // A finish says when; a start happens now and says nothing about a time it does not know.
        assert!(taken[0].0.occurred_at.is_none());
        assert_eq!(
            taken[1].0.occurred_at.as_deref(),
            Some("2026-07-23T00:00:05Z")
        );
    }

    /// A span transition names its span; a run transition does not.
    #[test]
    fn a_span_transition_names_its_span() {
        let local = Arc::new(CapturingExecutionTelemetry::default());
        let transitions = RecordingTransitions::watching(local.clone());
        let telemetry = CompositeExecutionTelemetry::new(local, Vec::new())
            .with_transitions(transitions.clone());
        let run = run();
        telemetry.start_run(&run).expect("start run");

        let span = ExecutionSpan {
            context: run.context.clone(),
            parent_span_id: None,
            name: "vanehub.task.execute".to_string(),
            status: ExecutionStatus::Running,
            fidelity: crate::contexts::execution_observability::domain::ExecutionFidelity::Native,
            started_at: "2026-07-23T00:00:01Z".to_string(),
            ended_at: None,
            error_classification: None,
            attributes: SafeAttributes::default(),
            links: Vec::new(),
        };
        telemetry.start_span(&span).expect("start span");

        let taken = transitions.taken();
        let (notice, _) = taken.last().expect("a span transition");
        assert_eq!(notice.kind, TraceTransitionKind::SpanStarted);
        assert_eq!(
            notice.span_id.as_deref(),
            Some(span.context.span_id.as_str())
        );
        // Identifiers only: the name the span carries never reaches the notice.
        assert_eq!(notice.run_id, run.context.run_id.as_str());
    }

    /// A transition for a run this process never started announces nothing.
    ///
    /// There is no correlation to announce it with, and publishing an empty trace id would hand a
    /// subscriber a value that looks like an id and matches nothing.
    #[test]
    fn a_transition_for_an_unknown_run_is_not_announced() {
        let local = Arc::new(CapturingExecutionTelemetry::default());
        let transitions = RecordingTransitions::watching(local.clone());
        let telemetry = CompositeExecutionTelemetry::new(local, Vec::new())
            .with_transitions(transitions.clone());

        telemetry
            .finish_run(
                &ExecutionRunId::parse("018f0f17-4d6a-7e20-b41d-66c5271a28ff").expect("run id"),
                ExecutionStatus::Succeeded,
                "2026-07-23T00:00:05Z",
                None,
            )
            .expect("finish run");

        assert!(transitions.taken().is_empty());
    }

    /// Telemetry assembled without a publisher still works.
    ///
    /// Several paths build telemetry with no window to publish to, and requiring one would make
    /// them carry a publisher that does nothing.
    #[test]
    fn telemetry_without_a_publisher_still_commits() {
        let local = Arc::new(CapturingExecutionTelemetry::default());
        let telemetry = CompositeExecutionTelemetry::new(local.clone(), Vec::new());

        telemetry.start_run(&run()).expect("start run");

        assert!(!local.records().expect("records").is_empty());
    }
}
