use crate::contexts::execution_observability::application::ExecutionTelemetryError;
use crate::contexts::execution_observability::domain::{
    ExecutionFidelity, ExecutionSpan, ExecutionStatus, SafeAttributeValue, SafeAttributes,
};
use opentelemetry::trace::{SpanId as OtelSpanId, TraceId as OtelTraceId};
use opentelemetry::KeyValue;
use opentelemetry_sdk::trace::{IdGenerator, RandomIdGenerator};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::SystemTime;

thread_local! {
    static PENDING_TRACE_IDS: RefCell<VecDeque<OtelTraceId>> = const { RefCell::new(VecDeque::new()) };
    static PENDING_SPAN_IDS: RefCell<VecDeque<OtelSpanId>> = const { RefCell::new(VecDeque::new()) };
}

#[derive(Debug, Default)]
pub(super) struct DomainIdGenerator {
    fallback: RandomIdGenerator,
}

impl IdGenerator for DomainIdGenerator {
    fn new_trace_id(&self) -> OtelTraceId {
        PENDING_TRACE_IDS
            .with(|ids| ids.borrow_mut().pop_front())
            .unwrap_or_else(|| self.fallback.new_trace_id())
    }

    fn new_span_id(&self) -> OtelSpanId {
        PENDING_SPAN_IDS
            .with(|ids| ids.borrow_mut().pop_front())
            .unwrap_or_else(|| self.fallback.new_span_id())
    }
}

pub(super) fn prepare_ids(span: &ExecutionSpan) -> Result<(), ExecutionTelemetryError> {
    let span_id = OtelSpanId::from_hex(span.context.span_id.as_str())
        .map_err(|error| unavailable(error.to_string()))?;
    PENDING_SPAN_IDS.with(|ids| ids.borrow_mut().push_back(span_id));
    if span.parent_span_id.is_none() {
        let trace_id = OtelTraceId::from_hex(span.context.trace_id.as_str())
            .map_err(|error| unavailable(error.to_string()))?;
        PENDING_TRACE_IDS.with(|ids| ids.borrow_mut().push_back(trace_id));
    }
    Ok(())
}

pub(super) fn span_key(span: &ExecutionSpan) -> (String, String) {
    (
        span.context.run_id.as_str().to_string(),
        span.context.span_id.as_str().to_string(),
    )
}

pub(super) fn otel_attributes(attributes: &SafeAttributes) -> Vec<KeyValue> {
    attributes
        .entries()
        .iter()
        .map(|(key, value)| match value {
            SafeAttributeValue::Boolean(value) => KeyValue::new(key.clone(), *value),
            SafeAttributeValue::Integer(value) => KeyValue::new(key.clone(), *value),
            SafeAttributeValue::Float(value) => KeyValue::new(key.clone(), *value),
            SafeAttributeValue::String(value) => KeyValue::new(key.clone(), value.clone()),
        })
        .collect()
}

pub(super) fn timestamp(value: &str) -> SystemTime {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|value| SystemTime::from(value.with_timezone(&chrono::Utc)))
        .unwrap_or_else(|_| SystemTime::now())
}

pub(super) fn fidelity_value(value: ExecutionFidelity) -> &'static str {
    match value {
        ExecutionFidelity::Native => "native",
        ExecutionFidelity::Proxied => "proxied",
        ExecutionFidelity::Inferred => "inferred",
        ExecutionFidelity::Opaque => "opaque",
    }
}

pub(super) fn status_value(value: ExecutionStatus) -> &'static str {
    match value {
        ExecutionStatus::Accepted => "accepted",
        ExecutionStatus::Running => "running",
        ExecutionStatus::Succeeded => "succeeded",
        ExecutionStatus::Failed => "failed",
        ExecutionStatus::Cancelled => "cancelled",
        ExecutionStatus::Incomplete => "incomplete",
    }
}

pub(super) fn unavailable(message: impl Into<String>) -> ExecutionTelemetryError {
    ExecutionTelemetryError::Unavailable(message.into())
}

pub(super) fn validated_endpoint(value: &str) -> Result<String, ExecutionTelemetryError> {
    let endpoint = url::Url::parse(value).map_err(|error| unavailable(error.to_string()))?;
    if !matches!(endpoint.scheme(), "http" | "https")
        || endpoint.host_str().is_none()
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.fragment().is_some()
    {
        return Err(unavailable(
            "OTLP endpoint must be an HTTP(S) URL without embedded credentials or fragments",
        ));
    }
    Ok(endpoint.to_string())
}
