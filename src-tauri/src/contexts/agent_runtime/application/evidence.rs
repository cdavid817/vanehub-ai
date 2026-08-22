/// What the agent runtime is willing to say about a run it executed.
///
/// This context already reports to `ExecutionTelemetryPort` using observability's own types. That
/// seam predates the journal and stays as it is; this one is deliberately different in kind — the
/// vocabulary is the runtime's, so the runtime never learns what an evidence event looks like.
///
/// Identifiers and one closed outcome. The prompt, the model's response, the tool arguments, and
/// the transcript are absent by construction rather than by filtering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentEvidenceSignal {
    RunStarted {
        session_id: String,
        run_id: String,
        trace_id: String,
        agent_id: Option<String>,
        seat_id: Option<String>,
        occurred_at: String,
    },
    RunFinished {
        session_id: String,
        run_id: String,
        trace_id: String,
        agent_id: Option<String>,
        seat_id: Option<String>,
        occurred_at: String,
        outcome: AgentRunEvidenceOutcome,
        /// Only when the runtime measured it. A duration inferred from two timestamps is a
        /// different quantity from how long the work took, and the journal keeps them apart.
        duration_ms: Option<u64>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentRunEvidenceOutcome {
    Succeeded,
    Failed,
    Cancelled,
}

/// Where the agent runtime hands an observation off.
///
/// Synchronous and infallible from the caller's side. A run that finished has finished; the
/// journal's availability is not part of that fact, and an agent operation must never fail, block,
/// or slow down because an observation could not be filed.
pub(crate) trait AgentEvidencePort: Send + Sync {
    fn try_publish(&self, signal: AgentEvidenceSignal);
}

pub(crate) struct NoAgentEvidence;

impl AgentEvidencePort for NoAgentEvidence {
    fn try_publish(&self, _signal: AgentEvidenceSignal) {}
}
