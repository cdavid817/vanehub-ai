use crate::contexts::execution_observability::application::ExecutionIdentityPort;
use crate::contexts::execution_observability::domain::{
    CapturePolicy, ExecutionContext, ExecutionRunId, SpanId, TraceId,
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RandomExecutionIdentity;

impl ExecutionIdentityPort for RandomExecutionIdentity {
    fn next_context(
        &self,
        capture_policy: CapturePolicy,
        sampling_ratio: f64,
        mcp_relay_enabled: bool,
    ) -> ExecutionContext {
        ExecutionContext {
            run_id: ExecutionRunId::parse(Uuid::new_v4().to_string())
                .unwrap_or_else(|_| unreachable!("UUID v4 always has the validated run id shape")),
            trace_id: TraceId::parse(Uuid::new_v4().simple().to_string()).unwrap_or_else(|_| {
                unreachable!("UUID v4 always has the validated W3C trace id shape")
            }),
            span_id: self.next_span_id(),
            capture_policy,
            sampling_per_million: (sampling_ratio * 1_000_000.0).round() as u32,
            mcp_relay_enabled,
        }
    }

    fn next_span_id(&self) -> SpanId {
        let value = Uuid::new_v4().simple().to_string();
        SpanId::parse(&value[..16])
            .unwrap_or_else(|_| unreachable!("UUID v4 prefix has the validated W3C span id shape"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_distinct_valid_w3c_contexts() {
        let identity = RandomExecutionIdentity;
        let first = identity.next_context(CapturePolicy::MetadataOnly, 1.0, false);
        let second = identity.next_context(CapturePolicy::MetadataOnly, 0.5, true);
        assert_ne!(first.run_id, second.run_id);
        assert_ne!(first.trace_id, second.trace_id);
        assert!(first.traceparent().starts_with("00-"));
        assert_eq!(first.traceparent().len(), 55);
        assert_eq!(first.sampling_per_million, 1_000_000);
        assert!(second.mcp_relay_enabled);
    }

    #[test]
    fn context_keeps_a_prospective_settings_snapshot() {
        let identity = RandomExecutionIdentity;
        let active = identity.next_context(CapturePolicy::MetadataOnly, 0.25, false);
        let later = identity.next_context(CapturePolicy::RedactedContent, 0.75, true);

        assert_eq!(active.capture_policy, CapturePolicy::MetadataOnly);
        assert_eq!(active.sampling_per_million, 250_000);
        assert!(!active.mcp_relay_enabled);
        assert_eq!(later.capture_policy, CapturePolicy::RedactedContent);
        assert_eq!(later.sampling_per_million, 750_000);
        assert!(later.mcp_relay_enabled);
    }
}
