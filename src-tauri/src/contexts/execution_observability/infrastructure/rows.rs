use super::storage_mapping::{
    parse_attributes, parse_capture, parse_fidelity, parse_source, parse_status, storage_error,
};
use crate::contexts::execution_observability::application::ExecutionTelemetryError;
use crate::contexts::execution_observability::domain::{
    ExecutionContext, ExecutionEvent, ExecutionFidelity, ExecutionLink, ExecutionRun,
    ExecutionRunId, ExecutionSource, ExecutionSpan, ExecutionStatus, SafeAttributes, SpanId,
    TraceId,
};
use rusqlite::Row;

pub(super) struct RunRow {
    run_id: String,
    trace_id: String,
    root_span_id: String,
    source: String,
    source_id: Option<String>,
    status: String,
    capture_policy: String,
    started_at: String,
    ended_at: Option<String>,
    error_classification: Option<String>,
    session_id: Option<String>,
    user_message_id: Option<String>,
    assistant_message_id: Option<String>,
    operation_id: Option<String>,
    agent_id: Option<String>,
    provider_session_id: Option<String>,
    attributes_json: String,
}

impl RunRow {
    pub(super) fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            run_id: row.get(0)?,
            trace_id: row.get(1)?,
            root_span_id: row.get(2)?,
            source: row.get(3)?,
            source_id: row.get(4)?,
            status: row.get(5)?,
            capture_policy: row.get(6)?,
            started_at: row.get(7)?,
            ended_at: row.get(8)?,
            error_classification: row.get(9)?,
            session_id: row.get(10)?,
            user_message_id: row.get(11)?,
            assistant_message_id: row.get(12)?,
            operation_id: row.get(13)?,
            agent_id: row.get(14)?,
            provider_session_id: row.get(15)?,
            attributes_json: row.get(16)?,
        })
    }

    pub(super) fn into_domain(
        self,
        links: Vec<ExecutionLink>,
    ) -> Result<ExecutionRun, ExecutionTelemetryError> {
        Ok(ExecutionRun {
            context: context(
                &self.run_id,
                &self.trace_id,
                &self.root_span_id,
                &self.capture_policy,
            )?,
            source: parse_source(&self.source, self.source_id)?,
            status: parse_status(&self.status)?,
            started_at: self.started_at,
            ended_at: self.ended_at,
            error_classification: self.error_classification,
            session_id: self.session_id,
            user_message_id: self.user_message_id,
            assistant_message_id: self.assistant_message_id,
            operation_id: self.operation_id,
            agent_id: self.agent_id,
            provider_session_id: self.provider_session_id,
            attributes: parse_attributes(&self.attributes_json)?,
            links,
        })
    }

    pub(super) fn cursor(&self) -> (&str, &str) {
        (&self.started_at, &self.run_id)
    }
}

pub(super) struct SpanRow {
    run_id: String,
    span_id: String,
    trace_id: String,
    parent_span_id: Option<String>,
    name: String,
    status: String,
    fidelity: String,
    started_at: String,
    ended_at: Option<String>,
    error_classification: Option<String>,
    attributes_json: String,
    capture_policy: String,
}

impl SpanRow {
    pub(super) fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            run_id: row.get(0)?,
            span_id: row.get(1)?,
            trace_id: row.get(2)?,
            parent_span_id: row.get(3)?,
            name: row.get(4)?,
            status: row.get(5)?,
            fidelity: row.get(6)?,
            started_at: row.get(7)?,
            ended_at: row.get(8)?,
            error_classification: row.get(9)?,
            attributes_json: row.get(10)?,
            capture_policy: row.get(11)?,
        })
    }

    pub(super) fn into_domain(
        self,
        links: Vec<ExecutionLink>,
    ) -> Result<ExecutionSpan, ExecutionTelemetryError> {
        Ok(ExecutionSpan {
            context: context(
                &self.run_id,
                &self.trace_id,
                &self.span_id,
                &self.capture_policy,
            )?,
            parent_span_id: self
                .parent_span_id
                .map(SpanId::parse)
                .transpose()
                .map_err(domain_error)?,
            name: self.name,
            status: parse_status(&self.status)?,
            fidelity: parse_fidelity(&self.fidelity)?,
            started_at: self.started_at,
            ended_at: self.ended_at,
            error_classification: self.error_classification,
            attributes: parse_attributes(&self.attributes_json)?,
            links,
        })
    }

    pub(super) fn span_id(&self) -> &str {
        &self.span_id
    }
}

pub(super) struct EventRow {
    run_id: String,
    span_id: String,
    sequence: u64,
    name: String,
    timestamp: String,
    attributes_json: String,
}

impl EventRow {
    pub(super) fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            run_id: row.get(0)?,
            span_id: row.get(1)?,
            sequence: row
                .get::<_, i64>(2)?
                .try_into()
                .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(2, -1))?,
            name: row.get(3)?,
            timestamp: row.get(4)?,
            attributes_json: row.get(5)?,
        })
    }

    pub(super) fn into_domain(self) -> Result<ExecutionEvent, ExecutionTelemetryError> {
        Ok(ExecutionEvent {
            run_id: ExecutionRunId::parse(self.run_id).map_err(domain_error)?,
            span_id: SpanId::parse(self.span_id).map_err(domain_error)?,
            sequence: self.sequence,
            name: self.name,
            timestamp: self.timestamp,
            attributes: parse_attributes(&self.attributes_json)?,
        })
    }
}

pub(super) fn link_from_parts(
    run_id: String,
    trace_id: String,
    span_id: Option<String>,
    relationship: String,
) -> Result<ExecutionLink, ExecutionTelemetryError> {
    Ok(ExecutionLink {
        run_id: ExecutionRunId::parse(run_id).map_err(domain_error)?,
        trace_id: TraceId::parse(trace_id).map_err(domain_error)?,
        span_id: span_id
            .map(SpanId::parse)
            .transpose()
            .map_err(domain_error)?,
        relationship,
    })
}

fn context(
    run_id: &str,
    trace_id: &str,
    span_id: &str,
    capture_policy: &str,
) -> Result<ExecutionContext, ExecutionTelemetryError> {
    Ok(ExecutionContext {
        run_id: ExecutionRunId::parse(run_id).map_err(domain_error)?,
        trace_id: TraceId::parse(trace_id).map_err(domain_error)?,
        span_id: SpanId::parse(span_id).map_err(domain_error)?,
        capture_policy: parse_capture(capture_policy)?,
        sampling_per_million: 1_000_000,
        mcp_relay_enabled: false,
    })
}

fn domain_error(error: impl std::fmt::Display) -> ExecutionTelemetryError {
    storage_error(error.to_string())
}

#[allow(dead_code)]
fn _type_guards(_: ExecutionStatus, _: ExecutionFidelity, _: ExecutionSource, _: SafeAttributes) {}
