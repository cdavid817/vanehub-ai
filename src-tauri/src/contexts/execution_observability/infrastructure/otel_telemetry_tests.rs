use super::*;
use crate::contexts::execution_observability::domain::{
    CapturePolicy, ExecutionContext, ExecutionFidelity, ExecutionRunId, SafeAttributes, TraceId,
};
use opentelemetry_sdk::trace::InMemorySpanExporter;

#[test]
fn exports_domain_trace_and_span_ids_with_parentage_and_safe_attributes() {
    let memory = InMemorySpanExporter::default();
    let exporter =
        OpenTelemetryExecutionExporter::with_span_exporter(memory.clone(), 1.0).expect("exporter");
    let root = span("00f067aa0ba902b7", None, "vanehub.task.execute");
    let child = span(
        "b7ad6b7169203331",
        Some(root.context.span_id.clone()),
        "invoke_agent codex-cli",
    );

    exporter.start_span(&root).expect("root");
    exporter.start_span(&child).expect("child");
    exporter
        .record_event(&ExecutionEvent {
            run_id: child.context.run_id.clone(),
            span_id: child.context.span_id.clone(),
            sequence: 1,
            name: "process.spawned".to_string(),
            timestamp: "2026-07-23T00:00:01Z".to_string(),
            attributes: SafeAttributes::default(),
        })
        .expect("event");
    exporter
        .finish_span(
            &child.context.run_id,
            &child.context.span_id,
            ExecutionStatus::Succeeded,
            "2026-07-23T00:00:02Z",
            None,
        )
        .expect("finish child");
    exporter
        .finish_span(
            &root.context.run_id,
            &root.context.span_id,
            ExecutionStatus::Succeeded,
            "2026-07-23T00:00:03Z",
            None,
        )
        .expect("finish root");
    exporter.tracer_provider.force_flush().expect("flush");

    let spans = memory.get_finished_spans().expect("finished spans");
    let exported_root = spans
        .iter()
        .find(|span| span.name == "vanehub.task.execute")
        .expect("exported root");
    let exported_child = spans
        .iter()
        .find(|span| span.name == "invoke_agent codex-cli")
        .expect("exported child");
    assert_eq!(
        exported_root.span_context.trace_id().to_string(),
        root.context.trace_id.as_str()
    );
    assert_eq!(
        exported_root.span_context.span_id().to_string(),
        root.context.span_id.as_str()
    );
    assert_eq!(
        exported_child.parent_span_id,
        exported_root.span_context.span_id()
    );
    assert_eq!(exported_child.events.len(), 1);
    assert!(spans
        .iter()
        .flat_map(|span| &span.attributes)
        .all(|attribute| !attribute.value.to_string().contains("secret prompt")));
}

#[test]
fn rejects_invalid_sampling_ratio() {
    assert!(OpenTelemetryExecutionExporter::with_span_exporter(
        InMemorySpanExporter::default(),
        1.1,
    )
    .is_err());
}

#[test]
fn rejects_unsafe_otlp_endpoints_before_exporter_creation() {
    assert!(validated_endpoint("file:///tmp/traces").is_err());
    assert!(validated_endpoint("https://token@example.com/v1/traces").is_err());
    assert!(validated_endpoint("https://example.com/v1/traces#secret").is_err());
    assert_eq!(
        validated_endpoint("https://collector.example.com/v1/traces").expect("endpoint"),
        "https://collector.example.com/v1/traces"
    );
}

fn span(span_id: &str, parent_span_id: Option<SpanId>, name: &str) -> ExecutionSpan {
    ExecutionSpan {
        context: ExecutionContext {
            run_id: ExecutionRunId::parse("018f0f17-4d6a-7e20-b41d-66c5271a28d0").expect("run id"),
            trace_id: TraceId::parse("4bf92f3577b34da6a3ce929d0e0e4736").expect("trace id"),
            span_id: SpanId::parse(span_id).expect("span id"),
            capture_policy: CapturePolicy::MetadataOnly,
            sampling_per_million: 1_000_000,
            mcp_relay_enabled: false,
        },
        parent_span_id,
        name: name.to_string(),
        status: ExecutionStatus::Running,
        fidelity: ExecutionFidelity::Native,
        started_at: "2026-07-23T00:00:00Z".to_string(),
        ended_at: None,
        error_classification: None,
        attributes: SafeAttributes::default(),
        links: Vec::new(),
    }
}
