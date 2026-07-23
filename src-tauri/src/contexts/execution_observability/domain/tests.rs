use super::*;

fn context() -> ExecutionContext {
    ExecutionContext {
        run_id: ExecutionRunId::parse("018f0f17-4d6a-7e20-b41d-66c5271a28d0").unwrap(),
        trace_id: TraceId::parse("4bf92f3577b34da6a3ce929d0e0e4736").unwrap(),
        span_id: SpanId::parse("00f067aa0ba902b7").unwrap(),
        capture_policy: CapturePolicy::MetadataOnly,
        sampling_per_million: 1_000_000,
        mcp_relay_enabled: false,
    }
}

#[test]
fn validates_w3c_identity_shapes_and_normalizes_case() {
    let trace = TraceId::parse("4BF92F3577B34DA6A3CE929D0E0E4736").unwrap();
    assert_eq!(trace.as_str(), "4bf92f3577b34da6a3ce929d0e0e4736");
    assert!(TraceId::parse("00000000000000000000000000000000").is_err());
    assert!(SpanId::parse("short").is_err());
    assert!(ExecutionRunId::parse("not-a-run-id").is_err());
}

#[test]
fn bounds_safe_attributes_and_pagination() {
    let value = SafeAttributeValue::bounded_string("safe").unwrap();
    let attributes = SafeAttributes::try_from_entries([("agent.id".to_string(), value)]).unwrap();
    assert_eq!(attributes.entries().len(), 1);
    assert!(SafeAttributeValue::bounded_string("x".repeat(257)).is_err());
    assert!(PageRequest::new(0, None).is_err());
    assert!(PageRequest::new(100, Some("cursor".to_string())).is_ok());
}

#[test]
fn validates_run_span_and_event_without_content_fields() {
    let run = ExecutionRun {
        context: context(),
        source: ExecutionSource::Desktop,
        status: ExecutionStatus::Running,
        started_at: "2026-07-23T00:00:00Z".to_string(),
        ended_at: None,
        error_classification: None,
        session_id: Some("session-1".to_string()),
        user_message_id: Some("message-1".to_string()),
        assistant_message_id: None,
        operation_id: Some("operation-1".to_string()),
        agent_id: Some("codex-cli".to_string()),
        provider_session_id: None,
        attributes: SafeAttributes::default(),
        links: Vec::new(),
    };
    assert!(run.validate().is_ok());
    assert!(!run.status.is_terminal());

    let span = ExecutionSpan {
        context: context(),
        parent_span_id: None,
        name: "invoke_agent codex-cli".to_string(),
        status: ExecutionStatus::Incomplete,
        fidelity: ExecutionFidelity::Inferred,
        started_at: run.started_at.clone(),
        ended_at: Some("2026-07-23T00:00:01Z".to_string()),
        error_classification: None,
        attributes: SafeAttributes::default(),
        links: Vec::new(),
    };
    assert!(span.validate().is_ok());
    assert!(span.status.is_terminal());

    let event = ExecutionEvent {
        run_id: context().run_id,
        span_id: context().span_id,
        sequence: 1,
        name: "process.spawned".to_string(),
        timestamp: run.started_at,
        attributes: SafeAttributes::default(),
    };
    assert!(event.validate().is_ok());
}

#[test]
fn exposes_all_fidelity_capture_and_source_values() {
    let fidelities = [
        ExecutionFidelity::Native,
        ExecutionFidelity::Proxied,
        ExecutionFidelity::Inferred,
        ExecutionFidelity::Opaque,
    ];
    assert_eq!(fidelities.len(), 4);
    assert_ne!(CapturePolicy::MetadataOnly, CapturePolicy::RedactedContent);
    assert!(matches!(
        ExecutionSource::InstantMessage {
            connector_id: "connector-1".to_string()
        },
        ExecutionSource::InstantMessage { .. }
    ));
}
