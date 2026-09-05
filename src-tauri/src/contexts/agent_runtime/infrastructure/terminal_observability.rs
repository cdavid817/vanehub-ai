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
        attributes: safe_attributes(agent_id, None),
        links: Vec::new(),
    });
    for span in [
        // The session and agent layers own the spans beneath them and do no work of their own.
        span(
            session.clone(),
            None,
            "vanehub.session.agent_terminal",
            ExecutionFidelity::Native,
            &started_at,
            safe_attributes(agent_id, Some(KIND_CONTAINER)),
        ),
        span(
            agent.clone(),
            Some(session.span_id.clone()),
            "vanehub.agent.terminal",
            ExecutionFidelity::Native,
            &started_at,
            safe_attributes(agent_id, Some(KIND_CONTAINER)),
        ),
        // A tool invocation whose interior stays unobservable. Both facts are stated: the kind says
        // what it is, the fidelity says VaneHub cannot see inside it, and neither implies the other.
        span(
            tool.clone(),
            Some(agent.span_id.clone()),
            "vanehub.tool.terminal_cli",
            ExecutionFidelity::Opaque,
            &started_at,
            safe_attributes(agent_id, Some(KIND_TOOL)),
        ),
        span(
            process.clone(),
            Some(tool.span_id.clone()),
            "vanehub.process.exec",
            ExecutionFidelity::Native,
            &started_at,
            safe_attributes(agent_id, Some(KIND_PROCESS)),
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

/// The attribute the classifier trusts above every convention.
///
/// A producer that knows its own kind is more authoritative than anything the classifier could
/// infer, and this producer knows all four. Without it the classifier falls through its table of
/// conventional attributes — none of which a terminal span carries — and answers `unknown`, which
/// is correct behavior against a producer that said nothing, and useless to a reader filtering the
/// timeline by kind.
const SPAN_KIND_ATTRIBUTE: &str = "vanehub.span.kind";

/// Kind tokens, matching what the classifier parses back into its enum.
///
/// Written as strings because that is the wire form of a span attribute. The tests assert the
/// classifier's answer rather than these literals, so a rename on either side fails there rather
/// than silently reintroducing `unknown`.
const KIND_CONTAINER: &str = "container";
const KIND_TOOL: &str = "tool";
const KIND_PROCESS: &str = "process";

/// Attributes for one span, optionally declaring what that span is.
///
/// `None` for the run: a run is not a span, and the classifier never reads it. Declaring a kind
/// there would be a value nothing consumes.
fn safe_attributes(agent_id: &str, kind: Option<&str>) -> SafeAttributes {
    let mut entries = vec![
        (
            "vanehub.stage".to_string(),
            SafeAttributeValue::String("agent_terminal".to_string()),
        ),
        (
            "vanehub.agent.id".to_string(),
            SafeAttributeValue::String(agent_id.to_string()),
        ),
    ];
    if let Some(kind) = kind {
        entries.push((
            SPAN_KIND_ATTRIBUTE.to_string(),
            SafeAttributeValue::String(kind.to_string()),
        ));
    }
    SafeAttributes::try_from_entries(entries).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::execution_observability::api::classify_span_kind;
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
        // Metadata only, and now exactly three keys: the agent and stage a reader filters by, plus
        // the kind the classifier reads. Asserted as a complete list rather than by membership so
        // that anything else a producer starts attaching has to be justified here first.
        assert_eq!(
            started[3]
                .attributes
                .entries()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                "vanehub.agent.id".to_string(),
                "vanehub.span.kind".to_string(),
                "vanehub.stage".to_string(),
            ]
        );
        assert!(matches!(
            records.last(),
            Some(CapturedTelemetryRecord::RunFinished {
                status: ExecutionStatus::Succeeded,
                ..
            })
        ));
    }

    /// Runs the emitted attributes through the real classifier rather than asserting the attribute
    /// map directly.
    ///
    /// Asserting the map would pass while the product stayed broken: every stage here was already
    /// individually correct, and the defect was that their composition produced `unknown` for all
    /// four spans. Only the classifier can say whether what the producer emitted is readable, so
    /// only the classifier's answer is worth asserting.
    #[test]
    fn declares_a_structured_kind_the_real_classifier_can_read() {
        let telemetry = CapturingExecutionTelemetry::default();
        start_terminal_execution_trace(
            &RandomExecutionIdentity,
            &telemetry,
            &FixedClock,
            &Default::default(),
            "session-1",
            "codex-cli",
            Some("provider-session-1"),
        );

        let records = telemetry.records().expect("telemetry records");
        let kinds = records
            .iter()
            .filter_map(|record| match record {
                CapturedTelemetryRecord::SpanStarted(span) => {
                    Some(classify_span_kind(&span.attributes).token())
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        // The session and agent layers own other spans and do no work themselves; the CLI layer is
        // a tool invocation; the exec layer is a process. A name like `vanehub.process.exec` is not
        // an assertion the classifier is allowed to read.
        assert_eq!(kinds, vec!["container", "container", "tool", "process"]);
    }

    /// Knowing what a span is and being able to see inside it are independent facts.
    ///
    /// Separate from the test above because a regression that dropped fidelity while keeping the
    /// kind would leave that one green, and the CLI's interior is exactly what VaneHub cannot
    /// observe. Declaring a kind must not quietly claim otherwise.
    #[test]
    fn declaring_a_kind_does_not_claim_visibility_into_the_cli() {
        let telemetry = CapturingExecutionTelemetry::default();
        start_terminal_execution_trace(
            &RandomExecutionIdentity,
            &telemetry,
            &FixedClock,
            &Default::default(),
            "session-1",
            "codex-cli",
            None,
        );

        let records = telemetry.records().expect("telemetry records");
        let tool = records
            .iter()
            .filter_map(|record| match record {
                CapturedTelemetryRecord::SpanStarted(span) => Some(span),
                _ => None,
            })
            .find(|span| span.name == "vanehub.tool.terminal_cli")
            .expect("the terminal CLI span");

        assert_eq!(classify_span_kind(&tool.attributes).token(), "tool");
        assert_eq!(tool.fidelity, ExecutionFidelity::Opaque);
    }

    /// Every span still carries the stage and agent attributes readers filter by.
    ///
    /// Adding a kind must not displace them: `MAX_ATTRIBUTE_COUNT` is generous, but a producer that
    /// swapped one attribute for another would break session filtering without failing any test
    /// that only looked at the kind.
    #[test]
    fn keeps_the_stage_and_agent_attributes_alongside_the_declared_kind() {
        let telemetry = CapturingExecutionTelemetry::default();
        start_terminal_execution_trace(
            &RandomExecutionIdentity,
            &telemetry,
            &FixedClock,
            &Default::default(),
            "session-1",
            "codex-cli",
            None,
        );

        let records = telemetry.records().expect("telemetry records");
        for record in &records {
            let CapturedTelemetryRecord::SpanStarted(span) = record else {
                continue;
            };
            let keys = span.attributes.entries();
            assert!(
                keys.contains_key("vanehub.stage"),
                "{} lost its stage",
                span.name
            );
            assert!(
                keys.contains_key("vanehub.agent.id"),
                "{} lost its agent id",
                span.name
            );
            assert!(
                keys.contains_key("vanehub.span.kind"),
                "{} declared no kind",
                span.name
            );
        }
    }
}
