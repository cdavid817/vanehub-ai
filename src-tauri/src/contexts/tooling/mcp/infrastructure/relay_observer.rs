use super::relay::RelayObservation;
use crate::contexts::execution_observability::api::{
    CapturePolicy, ExecutionContext, ExecutionFidelity, ExecutionRunId, ExecutionSpan,
    ExecutionStatus, ExecutionTelemetryPort, SafeAttributeValue, SafeAttributes, SpanId, TraceId,
};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub(super) struct RelayObserver {
    telemetry: Arc<dyn ExecutionTelemetryPort>,
    run_id: ExecutionRunId,
    trace_id: TraceId,
    parent_span_id: SpanId,
    capture_policy: CapturePolicy,
}

pub(super) struct RelayRequest {
    telemetry: Arc<dyn ExecutionTelemetryPort>,
    context: ExecutionContext,
    finished: bool,
}

impl RelayObserver {
    pub(super) fn new(
        observation: Option<&RelayObservation>,
        telemetry: Option<Arc<dyn ExecutionTelemetryPort>>,
    ) -> Option<Self> {
        let observation = observation?;
        Some(Self {
            telemetry: telemetry?,
            run_id: ExecutionRunId::parse(&observation.run_id).ok()?,
            trace_id: TraceId::parse(&observation.trace_id).ok()?,
            parent_span_id: SpanId::parse(&observation.parent_span_id).ok()?,
            capture_policy: match observation.capture_policy.as_str() {
                "redacted_content" => CapturePolicy::RedactedContent,
                _ => CapturePolicy::MetadataOnly,
            },
        })
    }

    pub(super) fn start_request(
        &self,
        transport: &str,
        method: Option<&str>,
    ) -> Option<RelayRequest> {
        let context = ExecutionContext {
            run_id: self.run_id.clone(),
            trace_id: self.trace_id.clone(),
            span_id: next_span_id(),
            capture_policy: self.capture_policy,
            sampling_per_million: 1_000_000,
            mcp_relay_enabled: true,
        };
        let attributes = SafeAttributes::try_from_entries([
            (
                "rpc.system".to_string(),
                SafeAttributeValue::String("mcp".to_string()),
            ),
            (
                "rpc.method".to_string(),
                SafeAttributeValue::String(method.unwrap_or("unknown").to_string()),
            ),
            (
                "network.transport".to_string(),
                SafeAttributeValue::String(transport.to_string()),
            ),
        ])
        .unwrap_or_default();
        self.telemetry
            .start_span(&ExecutionSpan {
                context: context.clone(),
                parent_span_id: Some(self.parent_span_id.clone()),
                name: "mcp.client.request".to_string(),
                status: ExecutionStatus::Running,
                fidelity: ExecutionFidelity::Proxied,
                started_at: now(),
                ended_at: None,
                error_classification: None,
                attributes,
                links: Vec::new(),
            })
            .ok()?;
        Some(RelayRequest {
            telemetry: self.telemetry.clone(),
            context,
            finished: false,
        })
    }

    pub(super) fn finish_request(
        &self,
        mut request: RelayRequest,
        success: bool,
        error_classification: Option<&str>,
    ) {
        let _ = request.telemetry.finish_span(
            &request.context.run_id,
            &request.context.span_id,
            if success {
                ExecutionStatus::Succeeded
            } else {
                ExecutionStatus::Failed
            },
            &now(),
            error_classification,
        );
        request.finished = true;
    }
}

impl Drop for RelayRequest {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        let _ = self.telemetry.finish_span(
            &self.context.run_id,
            &self.context.span_id,
            ExecutionStatus::Failed,
            &now(),
            Some("mcp_relay_terminated"),
        );
        self.finished = true;
    }
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn next_span_id() -> SpanId {
    let value = Uuid::new_v4().simple().to_string();
    SpanId::parse(&value[..16])
        .unwrap_or_else(|_| unreachable!("UUID v4 prefix always has a valid span id shape"))
}
