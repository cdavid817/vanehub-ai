use super::error::EvidenceDomainError;
use super::identity::{
    EvidenceAgentId, EvidenceCommandId, EvidenceFileMutationId, EvidenceOperationId,
    EvidenceSeatId, EvidenceSessionId, EvidenceToolCallId,
};
use crate::contexts::execution_observability::domain::{ExecutionRunId, SpanId, TraceId};

/// The references that let a reader get from one piece of evidence to every other view of the same
/// work. Everything except the session is optional, and an absent field stays absent: a fabricated
/// correlation is worse than none, because it links a record to work it did not belong to.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct EvidenceCorrelation {
    pub(crate) session_id: Option<EvidenceSessionId>,
    pub(crate) run_id: Option<ExecutionRunId>,
    pub(crate) trace_id: Option<TraceId>,
    pub(crate) span_id: Option<SpanId>,
    pub(crate) parent_span_id: Option<SpanId>,
    pub(crate) operation_id: Option<EvidenceOperationId>,
    pub(crate) agent_id: Option<EvidenceAgentId>,
    pub(crate) seat_id: Option<EvidenceSeatId>,
    pub(crate) tool_call_id: Option<EvidenceToolCallId>,
    pub(crate) command_id: Option<EvidenceCommandId>,
    pub(crate) file_mutation_id: Option<EvidenceFileMutationId>,
}

impl EvidenceCorrelation {
    pub(crate) fn for_session(session_id: EvidenceSessionId) -> Self {
        Self {
            session_id: Some(session_id),
            ..Self::default()
        }
    }

    /// Session is mandatory because it is the only scope every query is issued in; a record
    /// without one could never be found again, only counted.
    ///
    /// A span without its trace is rejected rather than repaired. W3C trace context makes a span
    /// id meaningful only inside a trace, so a lone span id is either a producer bug or a value
    /// copied from somewhere it did not belong, and both are better refused than persisted.
    pub(crate) fn validate(&self) -> Result<(), EvidenceDomainError> {
        if self.session_id.is_none() {
            return Err(EvidenceDomainError::SessionRequired);
        }
        if self.span_id.is_some() && self.trace_id.is_none() {
            return Err(EvidenceDomainError::SpanWithoutTrace);
        }
        if self.parent_span_id.is_some() && self.trace_id.is_none() {
            return Err(EvidenceDomainError::ParentSpanWithoutTrace);
        }
        Ok(())
    }

    pub(crate) fn session(&self) -> Option<&EvidenceSessionId> {
        self.session_id.as_ref()
    }

    /// The canonical rendering used by the idempotency fingerprint. Field order is fixed and every
    /// field is present, so two inputs that differ only in construction order fingerprint alike.
    pub(crate) fn canonical_parts(&self) -> Vec<(&'static str, String)> {
        vec![
            (
                "session",
                optional(self.session_id.as_ref().map(EvidenceSessionId::as_str)),
            ),
            (
                "run",
                optional(self.run_id.as_ref().map(ExecutionRunId::as_str)),
            ),
            (
                "trace",
                optional(self.trace_id.as_ref().map(TraceId::as_str)),
            ),
            ("span", optional(self.span_id.as_ref().map(SpanId::as_str))),
            (
                "parent_span",
                optional(self.parent_span_id.as_ref().map(SpanId::as_str)),
            ),
            (
                "operation",
                optional(self.operation_id.as_ref().map(EvidenceOperationId::as_str)),
            ),
            (
                "agent",
                optional(self.agent_id.as_ref().map(EvidenceAgentId::as_str)),
            ),
            (
                "seat",
                optional(self.seat_id.as_ref().map(EvidenceSeatId::as_str)),
            ),
            (
                "tool_call",
                optional(self.tool_call_id.as_ref().map(EvidenceToolCallId::as_str)),
            ),
            (
                "command",
                optional(self.command_id.as_ref().map(EvidenceCommandId::as_str)),
            ),
            (
                "file_mutation",
                optional(
                    self.file_mutation_id
                        .as_ref()
                        .map(EvidenceFileMutationId::as_str),
                ),
            ),
        ]
    }
}

fn optional(value: Option<&str>) -> String {
    value.unwrap_or("").to_string()
}
