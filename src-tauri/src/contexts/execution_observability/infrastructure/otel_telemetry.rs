use crate::contexts::execution_observability::application::{
    ExecutionTelemetryError, ExecutionTelemetryPort,
};
use crate::contexts::execution_observability::domain::{
    ExecutionEvent, ExecutionRun, ExecutionRunId, ExecutionSpan, ExecutionStatus, SpanId,
};
use opentelemetry::logs::{AnyValue, LogRecord, Logger, LoggerProvider, Severity};
use opentelemetry::metrics::{Counter, Meter, MeterProvider};
use opentelemetry::trace::{SpanKind, Status, TraceContextExt, Tracer, TracerProvider};
use opentelemetry::{Context, KeyValue};
use opentelemetry_otlp::{WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::logs::{SdkLogger, SdkLoggerProvider};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::{Sampler, SdkTracer, SdkTracerProvider, SpanExporter};
use opentelemetry_sdk::Resource;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::Duration;

type ActiveSpanMap = HashMap<(String, String), Context>;
type ActiveSpanGuard<'a> = std::sync::MutexGuard<'a, ActiveSpanMap>;

#[path = "otel_support.rs"]
mod otel_support;
use otel_support::{
    fidelity_value, otel_attributes, prepare_ids, span_key, status_value, timestamp, unavailable,
    validated_endpoint, DomainIdGenerator,
};

const INSTRUMENTATION_SCOPE: &str = "io.vanehub.execution";
const SCHEMA_VERSION: &str = "gen-ai/1.42.0";

#[derive(Clone)]
pub(crate) struct OpenTelemetryExecutionExporter {
    tracer: SdkTracer,
    tracer_provider: SdkTracerProvider,
    meter_provider: SdkMeterProvider,
    meter: Meter,
    logger_provider: Option<SdkLoggerProvider>,
    logger: Option<SdkLogger>,
    counters: Arc<Mutex<HashMap<&'static str, Counter<u64>>>>,
    spans: Arc<Mutex<ActiveSpanMap>>,
}

impl fmt::Debug for OpenTelemetryExecutionExporter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenTelemetryExecutionExporter")
            .finish_non_exhaustive()
    }
}

impl OpenTelemetryExecutionExporter {
    pub(crate) fn otlp_http(
        endpoint: &str,
        sampling_ratio: f64,
        timeout: Duration,
        auth_token: Option<&str>,
    ) -> Result<Self, ExecutionTelemetryError> {
        let endpoint = validated_endpoint(endpoint)?;
        let headers = auth_token.map(authorization_headers).unwrap_or_default();
        let span_exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_protocol(opentelemetry_otlp::Protocol::HttpBinary)
            .with_endpoint(endpoint.clone())
            .with_timeout(timeout)
            .with_headers(headers.clone())
            .build()
            .map_err(|error| unavailable(error.to_string()))?;
        let metric_exporter = opentelemetry_otlp::MetricExporter::builder()
            .with_http()
            .with_protocol(opentelemetry_otlp::Protocol::HttpBinary)
            .with_endpoint(endpoint.clone())
            .with_timeout(timeout)
            .with_headers(headers.clone())
            .build()
            .map_err(|error| unavailable(error.to_string()))?;
        let log_exporter = opentelemetry_otlp::LogExporter::builder()
            .with_http()
            .with_protocol(opentelemetry_otlp::Protocol::HttpBinary)
            .with_endpoint(endpoint)
            .with_timeout(timeout)
            .with_headers(headers)
            .build()
            .map_err(|error| unavailable(error.to_string()))?;
        let mut telemetry = Self::with_span_exporter(span_exporter, sampling_ratio)?;
        let resource = telemetry_resource();
        telemetry.meter_provider = SdkMeterProvider::builder()
            .with_resource(resource.clone())
            .with_periodic_exporter(metric_exporter)
            .build();
        telemetry.meter = telemetry.meter_provider.meter(INSTRUMENTATION_SCOPE);
        let logger_provider = SdkLoggerProvider::builder()
            .with_resource(resource)
            .with_batch_exporter(log_exporter)
            .build();
        telemetry.logger = Some(logger_provider.logger(INSTRUMENTATION_SCOPE));
        telemetry.logger_provider = Some(logger_provider);
        Ok(telemetry)
    }

    pub(crate) fn with_span_exporter<T: SpanExporter + 'static>(
        exporter: T,
        sampling_ratio: f64,
    ) -> Result<Self, ExecutionTelemetryError> {
        if !(0.0..=1.0).contains(&sampling_ratio) {
            return Err(unavailable("sampling ratio must be between 0 and 1"));
        }
        let resource = telemetry_resource();
        let tracer_provider = SdkTracerProvider::builder()
            .with_id_generator(DomainIdGenerator::default())
            .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
                sampling_ratio,
            ))))
            .with_max_attributes_per_span(32)
            .with_max_events_per_span(64)
            .with_batch_exporter(exporter)
            .with_resource(resource.clone())
            .build();
        let tracer = tracer_provider.tracer(INSTRUMENTATION_SCOPE);
        let meter_provider = SdkMeterProvider::builder().with_resource(resource).build();
        let meter = meter_provider.meter(INSTRUMENTATION_SCOPE);
        Ok(Self {
            tracer,
            tracer_provider,
            meter_provider,
            meter,
            logger_provider: None,
            logger: None,
            counters: Arc::new(Mutex::new(HashMap::new())),
            spans: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub(crate) fn shutdown(&self, timeout: Duration) -> Result<(), ExecutionTelemetryError> {
        self.tracer_provider
            .force_flush()
            .map_err(|error| unavailable(error.to_string()))?;
        self.tracer_provider
            .shutdown_with_timeout(timeout)
            .map_err(|error| unavailable(error.to_string()))?;
        self.meter_provider
            .shutdown_with_timeout(timeout)
            .map_err(|error| unavailable(error.to_string()))?;
        if let Some(provider) = &self.logger_provider {
            provider
                .shutdown_with_timeout(timeout)
                .map_err(|error| unavailable(error.to_string()))?;
        }
        Ok(())
    }
}

fn authorization_headers(token: &str) -> HashMap<String, String> {
    HashMap::from([("Authorization".to_string(), format!("Bearer {token}"))])
}

fn telemetry_resource() -> Resource {
    Resource::builder()
        .with_service_name("vanehub-ai")
        .with_attributes([
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            KeyValue::new("vanehub.telemetry.schema", SCHEMA_VERSION),
        ])
        .build()
}

impl crate::contexts::operations::api::ExternalLogExportPort for OpenTelemetryExecutionExporter {
    fn export_log(
        &self,
        log: &crate::contexts::operations::api::DiagnosticLog,
    ) -> Result<(), crate::contexts::operations::api::OperationsError> {
        let Some(logger) = &self.logger else {
            return Ok(());
        };
        let mut record = logger.create_log_record();
        let (severity, text) = match log.severity {
            crate::contexts::operations::api::LogSeverity::Error => (Severity::Error, "ERROR"),
            crate::contexts::operations::api::LogSeverity::Warn => (Severity::Warn, "WARN"),
            crate::contexts::operations::api::LogSeverity::Info => (Severity::Info, "INFO"),
            crate::contexts::operations::api::LogSeverity::Debug => (Severity::Debug, "DEBUG"),
        };
        record.set_severity_number(severity);
        record.set_severity_text(text);
        record.set_body(AnyValue::from(log.message.clone()));
        record.add_attribute("log.category", log.category.clone());
        record.add_attributes(log.context.clone());
        logger.emit(record);
        Ok(())
    }
}

impl ExecutionTelemetryPort for OpenTelemetryExecutionExporter {
    fn start_run(&self, _run: &ExecutionRun) -> Result<(), ExecutionTelemetryError> {
        Ok(())
    }

    fn finish_run(
        &self,
        _run_id: &ExecutionRunId,
        _status: ExecutionStatus,
        _ended_at: &str,
        _error_classification: Option<&str>,
    ) -> Result<(), ExecutionTelemetryError> {
        Ok(())
    }

    fn start_span(&self, span: &ExecutionSpan) -> Result<(), ExecutionTelemetryError> {
        prepare_ids(span)?;
        let parent = match &span.parent_span_id {
            Some(parent_span_id) => self
                .spans()?
                .get(&(
                    span.context.run_id.as_str().to_string(),
                    parent_span_id.as_str().to_string(),
                ))
                .cloned()
                .unwrap_or_default(),
            None => Context::new(),
        };
        let attributes = otel_attributes(&span.attributes)
            .into_iter()
            .chain([
                KeyValue::new("vanehub.run.id", span.context.run_id.as_str().to_string()),
                KeyValue::new("vanehub.fidelity", fidelity_value(span.fidelity)),
            ])
            .collect::<Vec<_>>();
        let builder = self
            .tracer
            .span_builder(span.name.clone())
            .with_kind(SpanKind::Internal)
            .with_start_time(timestamp(&span.started_at))
            .with_attributes(attributes);
        let otel_span = builder.start_with_context(&self.tracer, &parent);
        let context = Context::new().with_span(otel_span);
        self.spans()?.insert(span_key(span), context);
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
        let context = self
            .spans()?
            .remove(&(run_id.as_str().to_string(), span_id.as_str().to_string()))
            .ok_or_else(|| unavailable("OpenTelemetry span was not started"))?;
        let span = context.span();
        match status {
            ExecutionStatus::Succeeded => span.set_status(Status::Ok),
            ExecutionStatus::Failed => span.set_status(Status::error(
                error_classification
                    .unwrap_or("execution_failed")
                    .to_string(),
            )),
            ExecutionStatus::Cancelled | ExecutionStatus::Incomplete => {
                span.set_attribute(KeyValue::new(
                    "vanehub.execution.status",
                    status_value(status),
                ));
            }
            ExecutionStatus::Accepted | ExecutionStatus::Running => {}
        }
        if let Some(classification) = error_classification {
            span.set_attribute(KeyValue::new("error.type", classification.to_string()));
        }
        span.end_with_timestamp(timestamp(ended_at));
        Ok(())
    }

    fn record_event(&self, event: &ExecutionEvent) -> Result<(), ExecutionTelemetryError> {
        let spans = self.spans()?;
        let context = spans
            .get(&(
                event.run_id.as_str().to_string(),
                event.span_id.as_str().to_string(),
            ))
            .ok_or_else(|| unavailable("OpenTelemetry event parent span was not started"))?;
        context.span().add_event_with_timestamp(
            event.name.clone(),
            timestamp(&event.timestamp),
            otel_attributes(&event.attributes),
        );
        Ok(())
    }

    fn add_metric(
        &self,
        name: &'static str,
        value: u64,
        dimensions: &[(&'static str, &'static str)],
    ) -> Result<(), ExecutionTelemetryError> {
        let mut counters = self
            .counters
            .lock()
            .map_err(|error| unavailable(error.to_string()))?;
        let counter = counters
            .entry(name)
            .or_insert_with(|| self.meter.u64_counter(name).build())
            .clone();
        drop(counters);
        let attributes = safe_metric_dimensions(dimensions)
            .into_iter()
            .map(|(key, value)| KeyValue::new(key, value))
            .collect::<Vec<_>>();
        counter.add(value, &attributes);
        Ok(())
    }

    fn shutdown(&self, timeout: Duration) -> Result<(), ExecutionTelemetryError> {
        OpenTelemetryExecutionExporter::shutdown(self, timeout)
    }
}

fn safe_metric_dimensions(
    dimensions: &[(&'static str, &'static str)],
) -> Vec<(&'static str, &'static str)> {
    const ALLOWED: [&str; 7] = [
        "agent.id",
        "provider.id",
        "source",
        "outcome",
        "operation.class",
        "fidelity",
        "signal",
    ];
    dimensions
        .iter()
        .copied()
        .filter(|(key, _)| ALLOWED.contains(key))
        .collect()
}

#[cfg(test)]
mod metric_dimension_tests {
    use super::safe_metric_dimensions;

    #[test]
    fn excludes_all_high_cardinality_identity_dimensions() {
        let safe = safe_metric_dimensions(&[
            ("run.id", "run-1"),
            ("trace.id", "trace-1"),
            ("session.id", "session-1"),
            ("message.id", "message-1"),
            ("operation.id", "operation-1"),
            ("process.id", "42"),
            ("tool.call.id", "call-1"),
            ("outcome", "succeeded"),
        ]);
        assert_eq!(safe, vec![("outcome", "succeeded")]);
    }
}

impl OpenTelemetryExecutionExporter {
    fn spans(&self) -> Result<ActiveSpanGuard<'_>, ExecutionTelemetryError> {
        self.spans
            .lock()
            .map_err(|error| unavailable(error.to_string()))
    }
}

#[cfg(test)]
#[path = "otel_telemetry_tests.rs"]
mod tests;
