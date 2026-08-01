use crate::contexts::agent_runtime::application::AgentClockPort;
use crate::contexts::execution_observability::api::{
    ExecutionContext, ExecutionFidelity, ExecutionIdentityPort, ExecutionRun,
    ExecutionSettingsPort, ExecutionSource, ExecutionSpan, ExecutionStatus, ExecutionTelemetryPort,
    ObservabilitySettings, SafeAttributeValue, SafeAttributes, SpanId,
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct TerminalExecutionObservability {
    pub(super) execution_ids: Arc<dyn ExecutionIdentityPort>,
    pub(super) execution_settings: Arc<dyn ExecutionSettingsPort>,
    pub(super) telemetry: Arc<dyn ExecutionTelemetryPort>,
}

impl TerminalExecutionObservability {
    pub(crate) fn new(
        execution_ids: Arc<dyn ExecutionIdentityPort>,
        execution_settings: Arc<dyn ExecutionSettingsPort>,
        telemetry: Arc<dyn ExecutionTelemetryPort>,
    ) -> Self {
        Self {
            execution_ids,
            execution_settings,
            telemetry,
        }
    }
}

pub(super) struct TerminalExecutionTrace {
    pub(super) session: ExecutionContext,
    agent: ExecutionContext,
    tool: ExecutionContext,
    pub(super) process: ExecutionContext,
}

pub(super) fn start_terminal_execution_trace(
    execution_ids: &dyn ExecutionIdentityPort,
    telemetry: &dyn ExecutionTelemetryPort,
    clock: &dyn AgentClockPort,
    settings: &ObservabilitySettings,
    session_id: &str,
    agent_id: &str,
    provider_session_id: Option<&str>,
) -> TerminalExecutionTrace {
    let started_at = clock.now();
    let session = execution_ids.next_context(
        settings.capture_policy,
        settings.sampling_ratio,
        settings.mcp_relay_enabled,
    );
    let agent = child_context(&session, execution_ids.next_span_id());
    let tool = child_context(&agent, execution_ids.next_span_id());
    let process = child_context(&tool, execution_ids.next_span_id());
    let attributes = safe_attributes(agent_id);
    let _ = telemetry.start_run(&ExecutionRun {
        context: session.clone(),
        source: ExecutionSource::Desktop,
        status: ExecutionStatus::Running,
        started_at: started_at.clone(),
        ended_at: None,
        error_classification: None,
        session_id: Some(session_id.to_string()),
        user_message_id: None,
        assistant_message_id: None,
        operation_id: None,
        agent_id: Some(agent_id.to_string()),
        provider_session_id: provider_session_id.map(str::to_string),
        attributes: attributes.clone(),
        links: Vec::new(),
    });
    for span in [
        span(
            session.clone(),
            None,
            "vanehub.session.agent_terminal",
            ExecutionFidelity::Native,
            &started_at,
            attributes.clone(),
        ),
        span(
            agent.clone(),
            Some(session.span_id.clone()),
            "vanehub.agent.terminal",
            ExecutionFidelity::Native,
            &started_at,
            attributes.clone(),
        ),
        span(
            tool.clone(),
            Some(agent.span_id.clone()),
            "vanehub.tool.terminal_cli",
            ExecutionFidelity::Opaque,
            &started_at,
            attributes.clone(),
        ),
        span(
            process.clone(),
            Some(tool.span_id.clone()),
            "vanehub.process.exec",
            ExecutionFidelity::Native,
            &started_at,
            attributes,
        ),
    ] {
        let _ = telemetry.start_span(&span);
    }
    TerminalExecutionTrace {
        session,
        agent,
        tool,
        process,
    }
}

pub(super) fn finish_terminal_execution_trace(
    telemetry: &dyn ExecutionTelemetryPort,
    clock: &dyn AgentClockPort,
    trace: &TerminalExecutionTrace,
    status: ExecutionStatus,
    error_classification: Option<&str>,
) {
    let ended_at = clock.now();
    for context in [&trace.process, &trace.tool, &trace.agent, &trace.session] {
        let _ = telemetry.finish_span(
            &context.run_id,
            &context.span_id,
            status,
            &ended_at,
            error_classification,
        );
    }
    let _ = telemetry.finish_run(
        &trace.session.run_id,
        status,
        &ended_at,
        error_classification,
    );
}

fn span(
    context: ExecutionContext,
    parent_span_id: Option<SpanId>,
    name: &str,
    fidelity: ExecutionFidelity,
    started_at: &str,
    attributes: SafeAttributes,
) -> ExecutionSpan {
    ExecutionSpan {
        context,
        parent_span_id,
        name: name.to_string(),
        status: ExecutionStatus::Running,
        fidelity,
        started_at: started_at.to_string(),
        ended_at: None,
        error_classification: None,
        attributes,
        links: Vec::new(),
    }
}

fn child_context(parent: &ExecutionContext, span_id: SpanId) -> ExecutionContext {
    ExecutionContext {
        run_id: parent.run_id.clone(),
        trace_id: parent.trace_id.clone(),
        span_id,
        capture_policy: parent.capture_policy,
        sampling_per_million: parent.sampling_per_million,
        mcp_relay_enabled: parent.mcp_relay_enabled,
    }
}

fn safe_attributes(agent_id: &str) -> SafeAttributes {
    SafeAttributes::try_from_entries([
        (
            "vanehub.stage".to_string(),
            SafeAttributeValue::String("agent_terminal".to_string()),
        ),
        (
            "vanehub.agent.id".to_string(),
            SafeAttributeValue::String(agent_id.to_string()),
        ),
    ])
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::execution_observability::application::test_adapter::{
        CapturedTelemetryRecord, CapturingExecutionTelemetry,
    };
    use crate::contexts::execution_observability::infrastructure::RandomExecutionIdentity;

    struct FixedClock;

    impl AgentClockPort for FixedClock {
        fn now(&self) -> String {
            "2026-08-01T10:00:00+00:00".to_string()
        }
    }

    #[test]
    fn preserves_session_agent_tool_process_hierarchy() {
        let telemetry = CapturingExecutionTelemetry::default();
        let trace = start_terminal_execution_trace(
            &RandomExecutionIdentity,
            &telemetry,
            &FixedClock,
            &Default::default(),
            "session-1",
            "codex-cli",
            Some("provider-session-1"),
        );
        finish_terminal_execution_trace(
            &telemetry,
            &FixedClock,
            &trace,
            ExecutionStatus::Succeeded,
            None,
        );

        let records = telemetry.records().expect("telemetry records");
        let started = records
            .iter()
            .filter_map(|record| match record {
                CapturedTelemetryRecord::SpanStarted(span) => Some(span),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(started.len(), 4);
        assert_eq!(started[0].name, "vanehub.session.agent_terminal");
        assert_eq!(started[0].parent_span_id, None);
        assert_eq!(started[1].name, "vanehub.agent.terminal");
        assert_eq!(
            started[1].parent_span_id,
            Some(started[0].context.span_id.clone())
        );
        assert_eq!(started[2].name, "vanehub.tool.terminal_cli");
        assert_eq!(started[2].fidelity, ExecutionFidelity::Opaque);
        assert_eq!(
            started[2].parent_span_id,
            Some(started[1].context.span_id.clone())
        );
        assert_eq!(started[3].name, "vanehub.process.exec");
        assert_eq!(
            started[3].parent_span_id,
            Some(started[2].context.span_id.clone())
        );
        assert_eq!(
            started[3]
                .attributes
                .entries()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec!["vanehub.agent.id".to_string(), "vanehub.stage".to_string()]
        );
        assert!(matches!(
            records.last(),
            Some(CapturedTelemetryRecord::RunFinished {
                status: ExecutionStatus::Succeeded,
                ..
            })
        ));
    }
}
