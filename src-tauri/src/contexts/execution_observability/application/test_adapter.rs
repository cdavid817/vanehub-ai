use super::{ExecutionTelemetryError, ExecutionTelemetryPort};
use crate::contexts::execution_observability::domain::{
    ExecutionEvent, ExecutionRun, ExecutionRunId, ExecutionSpan, ExecutionStatus, SpanId,
};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CapturedTelemetryRecord {
    RunStarted(ExecutionRun),
    RunFinished {
        run_id: ExecutionRunId,
        status: ExecutionStatus,
        ended_at: String,
        error_classification: Option<String>,
    },
    SpanStarted(ExecutionSpan),
    SpanFinished {
        run_id: ExecutionRunId,
        span_id: SpanId,
        status: ExecutionStatus,
        ended_at: String,
        error_classification: Option<String>,
    },
    Event(ExecutionEvent),
    Metric {
        name: &'static str,
        value: u64,
        dimensions: Vec<(&'static str, &'static str)>,
    },
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CapturingExecutionTelemetry {
    records: Arc<Mutex<Vec<CapturedTelemetryRecord>>>,
}

impl CapturingExecutionTelemetry {
    pub(crate) fn records(&self) -> Result<Vec<CapturedTelemetryRecord>, ExecutionTelemetryError> {
        self.records
            .lock()
            .map(|records| records.clone())
            .map_err(|error| ExecutionTelemetryError::Unavailable(error.to_string()))
    }

    fn push(&self, record: CapturedTelemetryRecord) -> Result<(), ExecutionTelemetryError> {
        self.records
            .lock()
            .map_err(|error| ExecutionTelemetryError::Unavailable(error.to_string()))?
            .push(record);
        Ok(())
    }
}

impl ExecutionTelemetryPort for CapturingExecutionTelemetry {
    fn start_run(&self, run: &ExecutionRun) -> Result<(), ExecutionTelemetryError> {
        self.push(CapturedTelemetryRecord::RunStarted(run.clone()))
    }

    fn finish_run(
        &self,
        run_id: &ExecutionRunId,
        status: ExecutionStatus,
        ended_at: &str,
        error_classification: Option<&str>,
    ) -> Result<(), ExecutionTelemetryError> {
        self.push(CapturedTelemetryRecord::RunFinished {
            run_id: run_id.clone(),
            status,
            ended_at: ended_at.to_string(),
            error_classification: error_classification.map(str::to_string),
        })
    }

    fn start_span(&self, span: &ExecutionSpan) -> Result<(), ExecutionTelemetryError> {
        self.push(CapturedTelemetryRecord::SpanStarted(span.clone()))
    }

    fn finish_span(
        &self,
        run_id: &ExecutionRunId,
        span_id: &SpanId,
        status: ExecutionStatus,
        ended_at: &str,
        error_classification: Option<&str>,
    ) -> Result<(), ExecutionTelemetryError> {
        self.push(CapturedTelemetryRecord::SpanFinished {
            run_id: run_id.clone(),
            span_id: span_id.clone(),
            status,
            ended_at: ended_at.to_string(),
            error_classification: error_classification.map(str::to_string),
        })
    }

    fn record_event(&self, event: &ExecutionEvent) -> Result<(), ExecutionTelemetryError> {
        self.push(CapturedTelemetryRecord::Event(event.clone()))
    }

    fn add_metric(
        &self,
        name: &'static str,
        value: u64,
        dimensions: &[(&'static str, &'static str)],
    ) -> Result<(), ExecutionTelemetryError> {
        self.push(CapturedTelemetryRecord::Metric {
            name,
            value,
            dimensions: dimensions.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::execution_observability::domain::{
        CapturePolicy, ExecutionContext, ExecutionSource, SafeAttributes, TraceId,
    };

    #[test]
    fn captures_records_in_call_order() {
        let adapter = CapturingExecutionTelemetry::default();
        let run_id = ExecutionRunId::parse("018f0f17-4d6a-7e20-b41d-66c5271a28d0").unwrap();
        let run = ExecutionRun {
            context: ExecutionContext {
                run_id: run_id.clone(),
                trace_id: TraceId::parse("4bf92f3577b34da6a3ce929d0e0e4736").unwrap(),
                span_id: SpanId::parse("00f067aa0ba902b7").unwrap(),
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
        };

        adapter.start_run(&run).unwrap();
        adapter
            .finish_run(
                &run_id,
                ExecutionStatus::Succeeded,
                "2026-07-23T00:00:01Z",
                None,
            )
            .unwrap();

        let records = adapter.records().unwrap();
        assert!(matches!(records[0], CapturedTelemetryRecord::RunStarted(_)));
        assert!(matches!(
            records[1],
            CapturedTelemetryRecord::RunFinished {
                status: ExecutionStatus::Succeeded,
                ..
            }
        ));
    }
}
