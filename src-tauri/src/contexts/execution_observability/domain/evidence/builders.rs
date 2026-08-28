use super::identity::{BoundedLabel, EvidenceAgentId, EvidenceFileMutationId};
use super::{
    reason_codes, EvidenceCommandId, EvidenceCorrelation, EvidenceCoverageState, EvidenceEventId,
    EvidenceOperationId, EvidenceSeatId, EvidenceSessionId, EvidenceSourceContext,
    EvidenceToolCallId, ExecutionEvidenceEvent, ExecutionEvidenceEventInput, QueryCoverage,
    RedactionReceipt, SafeEvidencePayload, SafeReasonCode, SourceEventId, EVIDENCE_SCHEMA_VERSION,
};
use crate::contexts::execution_observability::domain::{
    ExecutionFidelity, ExecutionRunId, ExecutionStatus, TraceId,
};

/// Builders for evidence fixtures.
///
/// Every value goes through the real domain constructor rather than being assembled field by
/// field. A builder that bypassed validation would let a test assert on a shape the production
/// path can never produce, which is worse than having no builder: the suite would stay green while
/// the invariant it claims to cover was broken.
pub(crate) struct CorrelationBuilder {
    correlation: EvidenceCorrelation,
}

impl CorrelationBuilder {
    pub(crate) fn for_session(session_id: &str) -> Self {
        Self {
            correlation: EvidenceCorrelation::for_session(
                EvidenceSessionId::parse(session_id).expect("valid session id"),
            ),
        }
    }

    /// A run always brings its trace: the two are recorded together in production, and a fixture
    /// that separated them would exercise a state the runtime cannot reach.
    pub(crate) fn with_run(mut self, run_id: &str, trace_id: &str) -> Self {
        self.correlation.run_id = Some(ExecutionRunId::parse(run_id).expect("valid run id"));
        self.correlation.trace_id = Some(TraceId::parse(trace_id).expect("valid trace id"));
        self
    }

    pub(crate) fn with_operation(mut self, operation_id: &str) -> Self {
        self.correlation.operation_id =
            Some(EvidenceOperationId::parse(operation_id).expect("valid operation id"));
        self
    }

    pub(crate) fn with_agent(mut self, agent_id: &str) -> Self {
        self.correlation.agent_id = Some(EvidenceAgentId::parse(agent_id).expect("valid agent id"));
        self
    }

    pub(crate) fn with_seat(mut self, seat_id: &str) -> Self {
        self.correlation.seat_id = Some(EvidenceSeatId::parse(seat_id).expect("valid seat id"));
        self
    }

    pub(crate) fn with_tool_call(mut self, tool_call_id: &str) -> Self {
        self.correlation.tool_call_id =
            Some(EvidenceToolCallId::parse(tool_call_id).expect("valid tool call id"));
        self
    }

    pub(crate) fn with_command(mut self, command_id: &str) -> Self {
        self.correlation.command_id =
            Some(EvidenceCommandId::parse(command_id).expect("valid command id"));
        self
    }

    pub(crate) fn with_file_mutation(mut self, file_mutation_id: &str) -> Self {
        self.correlation.file_mutation_id =
            Some(EvidenceFileMutationId::parse(file_mutation_id).expect("valid file mutation id"));
        self
    }

    /// Runs the same validation the production path runs, so a fixture that violates an invariant
    /// fails here instead of producing an impossible value the test then trusts.
    pub(crate) fn build(self) -> EvidenceCorrelation {
        self.correlation.validate().expect("valid correlation");
        self.correlation
    }

    /// For the cases that exist to prove an invariant rejects; skips the assertion, not the check.
    pub(crate) fn build_unchecked(self) -> EvidenceCorrelation {
        self.correlation
    }
}

pub(crate) struct EvidenceEventBuilder {
    input: ExecutionEvidenceEventInput,
}

impl EvidenceEventBuilder {
    pub(crate) fn new(
        source_event_id: &str,
        correlation: EvidenceCorrelation,
        payload: SafeEvidencePayload,
    ) -> Self {
        Self {
            input: ExecutionEvidenceEventInput {
                event_id: EvidenceEventId::parse(format!("event-{source_event_id}"))
                    .expect("valid event id"),
                source_context: EvidenceSourceContext::AgentRuntime,
                source_event_id: SourceEventId::parse(source_event_id)
                    .expect("valid source event id"),
                schema_version: EVIDENCE_SCHEMA_VERSION,
                occurred_at: "2026-01-01T00:00:00.000Z".to_string(),
                correlation,
                status: None,
                fidelity: ExecutionFidelity::Native,
                payload,
                redaction: RedactionReceipt::none(),
            },
        }
    }

    pub(crate) fn with_event_id(mut self, event_id: &str) -> Self {
        self.input.event_id = EvidenceEventId::parse(event_id).expect("valid event id");
        self
    }

    pub(crate) fn with_source(mut self, source_context: EvidenceSourceContext) -> Self {
        self.input.source_context = source_context;
        self
    }

    pub(crate) fn with_occurred_at(mut self, occurred_at: &str) -> Self {
        self.input.occurred_at = occurred_at.to_string();
        self
    }

    pub(crate) fn with_status(mut self, status: ExecutionStatus) -> Self {
        self.input.status = Some(status);
        self
    }

    pub(crate) fn with_fidelity(mut self, fidelity: ExecutionFidelity) -> Self {
        self.input.fidelity = fidelity;
        self
    }

    pub(crate) fn with_redaction(mut self, rule_ids: &[&str]) -> Self {
        self.input.redaction = RedactionReceipt::applied(
            rule_ids
                .iter()
                .map(|rule| SafeReasonCode::parse(*rule).expect("valid rule id")),
        )
        .expect("bounded redaction receipt");
        self
    }

    pub(crate) fn build(self) -> ExecutionEvidenceEvent {
        ExecutionEvidenceEvent::new(self.input).expect("valid evidence event")
    }

    pub(crate) fn try_build(self) -> Result<ExecutionEvidenceEvent, super::EvidenceDomainError> {
        ExecutionEvidenceEvent::new(self.input)
    }
}

/// Coverage fixtures for each of the four states, built through the real constructor so a reason
/// code that is not a valid `SafeReasonCode` fails the fixture rather than the assertion.
pub(crate) struct CoverageBuilder;

impl CoverageBuilder {
    pub(crate) fn complete() -> QueryCoverage {
        QueryCoverage::complete()
    }

    pub(crate) fn indexing() -> QueryCoverage {
        QueryCoverage::new(
            EvidenceCoverageState::Indexing,
            [SafeReasonCode::parse(reason_codes::PROJECTION_REBUILDING).expect("valid reason")],
        )
        .expect("bounded coverage")
    }

    pub(crate) fn partial(reason: &str) -> QueryCoverage {
        QueryCoverage::new(
            EvidenceCoverageState::Partial,
            [SafeReasonCode::parse(reason).expect("valid reason")],
        )
        .expect("bounded coverage")
    }

    pub(crate) fn unavailable(reason: &str) -> QueryCoverage {
        QueryCoverage::new(
            EvidenceCoverageState::Unavailable,
            [SafeReasonCode::parse(reason).expect("valid reason")],
        )
        .expect("bounded coverage")
    }

    /// The state a production store is in before Task Group 4 wires any producer: queryable, but
    /// with nothing capturing into it, so an empty answer is not evidence of absence.
    pub(crate) fn capture_not_initialized() -> QueryCoverage {
        Self::partial(reason_codes::CAPTURE_NOT_INITIALIZED)
    }
}

pub(crate) fn label(value: &str) -> BoundedLabel {
    BoundedLabel::parse("fixture", value).expect("valid label")
}

pub(crate) fn reason(value: &str) -> SafeReasonCode {
    SafeReasonCode::parse(value).expect("valid reason code")
}
