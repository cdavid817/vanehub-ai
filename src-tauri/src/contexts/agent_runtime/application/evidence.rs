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
    /// A tool call the runtime saw begin. `attempt` distinguishes a retry of the same call from a
    /// new one, which is what keeps a retried tool from reading as two separate invocations.
    ToolStarted {
        session_id: String,
        run_id: String,
        trace_id: String,
        span_id: Option<String>,
        agent_id: Option<String>,
        seat_id: Option<String>,
        call_id: String,
        tool_name: String,
        /// Where the runtime saw it: directly, through a provider's report, or reconstructed.
        /// Never upgraded — a CLI adapter that parsed a line has not observed what a native tool
        /// loop observes, and recording both as `native` would erase that difference.
        observation: AgentEvidenceObservation,
        attempt: Option<u32>,
        occurred_at: String,
    },
    ToolFinished {
        session_id: String,
        run_id: String,
        trace_id: String,
        span_id: Option<String>,
        agent_id: Option<String>,
        seat_id: Option<String>,
        call_id: String,
        tool_name: String,
        observation: AgentEvidenceObservation,
        attempt: Option<u32>,
        outcome: AgentRunEvidenceOutcome,
        occurred_at: String,
    },
    /// A tool call that handed work to another agent. Carried separately from `ToolStarted`
    /// because a delegation is a relationship between two agents, and a reader looking for "what
    /// did this agent hand off" cannot find it in a list of tool names.
    DelegationStarted {
        session_id: String,
        run_id: String,
        trace_id: String,
        span_id: Option<String>,
        parent_agent_id: Option<String>,
        seat_id: Option<String>,
        delegation_id: String,
        call_id: String,
        attempt: Option<u32>,
        occurred_at: String,
    },
    DelegationFinished {
        session_id: String,
        run_id: String,
        trace_id: String,
        span_id: Option<String>,
        parent_agent_id: Option<String>,
        seat_id: Option<String>,
        delegation_id: String,
        call_id: String,
        attempt: Option<u32>,
        outcome: AgentRunEvidenceOutcome,
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

/// How directly the runtime saw what it is reporting.
///
/// A CLI adapter that recognised a line in stdout and a native tool loop that dispatched the call
/// itself are not the same observation, and a consumer that cannot tell them apart will present
/// both as equally certain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentEvidenceObservation {
    /// The runtime dispatched it and watched it return.
    Direct,
    /// A provider reported it over a structured protocol.
    Reported,
    /// Reconstructed from output the runtime did not originate.
    Reconstructed,
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

/// A publisher that records nothing.
///
/// Test-only. Production takes its publisher as a constructor argument, so an assembly that
/// forgets one fails to compile rather than running and quietly recording nothing — which used to
/// surface as a panel reporting that a session did no work.
#[cfg(test)]
pub(crate) struct NoAgentEvidence;

#[cfg(test)]
impl AgentEvidencePort for NoAgentEvidence {
    fn try_publish(&self, _signal: AgentEvidenceSignal) {}
}
