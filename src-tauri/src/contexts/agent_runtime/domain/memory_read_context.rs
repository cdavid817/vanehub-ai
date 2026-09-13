/// Where the session's workspace stands for memory purposes, as the owning context resolved it.
///
/// Three answers on purpose. "Explicitly no workspace" may still read global records under
/// policy; "there is one but it could not be resolved" reads nothing, because no scope decision is
/// safe without knowing which workspace the records would have to match.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum AgentWorkspaceBinding {
    Absent,
    Resolved(String),
    Unresolved,
}

/// The trusted memory-read authority for one generation, carried by the runtime, minted only by
/// the native personalization owner.
///
/// The runtime never builds or edits one: every field is copied from the owning context's frozen
/// value, and the fingerprint binds them to that context's process epoch so the owner can refuse
/// anything assembled elsewhere. Every memory surface -- index, selector, bodies, recall, Context
/// Engine -- carries the same value, which is what makes their eligibility decisions identical.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct AgentMemoryReadContext {
    pub(crate) agent_id: String,
    pub(crate) session_id: String,
    pub(crate) generation_id: String,
    pub(crate) seat_id: Option<String>,
    pub(crate) workspace: AgentWorkspaceBinding,
    pub(crate) session_mode: String,
    pub(crate) policy_revision: String,
    pub(crate) read: bool,
    pub(crate) global_allowed: bool,
    pub(crate) workspace_allowed: Option<String>,
    pub(crate) maintenance_generation: u64,
    pub(crate) contract_version: u32,
    pub(crate) fingerprint: String,
}

impl AgentMemoryReadContext {
    /// Whether any memory surface may do work at all. False for temporary and read-disabled
    /// sessions and for an unresolved workspace, so every consumer can skip -- no selector call,
    /// no query embedding, no body load -- with one question.
    pub(crate) fn permits_read(&self) -> bool {
        self.read && (self.global_allowed || self.workspace_allowed.is_some())
    }

    /// The subject a per-session cache is partitioned by: the actual Agent and seat plus the
    /// frozen scope, so one seat's surfaced markers never suppress or authorize another's.
    pub(crate) fn subject_key(&self) -> String {
        let workspace = match &self.workspace {
            AgentWorkspaceBinding::Absent => "absent".to_string(),
            AgentWorkspaceBinding::Resolved(key) => format!("ws:{key}"),
            AgentWorkspaceBinding::Unresolved => "unresolved".to_string(),
        };
        format!(
            "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
            self.agent_id,
            self.seat_id.clone().unwrap_or_default(),
            workspace,
            self.session_mode,
            self.global_allowed,
            self.workspace_allowed.clone().unwrap_or_default(),
        )
    }
}
