//! A tool call awaiting a human's approve/deny decision.

use super::action::Action;
use super::effect::Effect;
use super::resource::Resource;
use super::risk_level::RiskLevel;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillApprovalProvenance {
    pub(crate) parent_agent_id: String,
    pub(crate) skill_id: String,
    pub(crate) tool_id: String,
    pub(crate) effective_revision: String,
    pub(crate) source_scope: String,
    pub(crate) requested_capability: String,
    pub(crate) delegated_operation: String,
    pub(crate) redacted_input_summary: String,
    pub(crate) immutable_witness: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum SkillApprovalInvalidation {
    Cancellation,
    RevisionReplaced,
    Disabled,
    Quarantined,
    WitnessMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApprovalRequest {
    pub(crate) id: String,
    pub(crate) principal_id: String,
    pub(crate) agent_id: String,
    pub(crate) session_id: String,
    pub(crate) generation_id: String,
    /// Correlates back to the native tool-use loop's own pending-call bookkeeping
    /// (`RuntimeAgentApiAdapter`'s `pending_approvals`, keyed by this same id) — opaque to
    /// `permissions` itself, just carried through.
    pub(crate) call_id: String,
    /// Captured at creation time so resolving a `Scope::Project` grant later never needs to
    /// re-derive "which project is this" by reaching into `sessions`/`workspaces`.
    pub(crate) project_key: String,
    pub(crate) action: Action,
    pub(crate) resource: Resource,
    pub(crate) risk_level: RiskLevel,
    pub(crate) skill: Option<SkillApprovalProvenance>,
    pub(crate) created_at: String,
}

/// A human's resolution of a pending approval. Deliberately not the three-valued `Effect` — a
/// human can approve or deny, never resolve something to "still ask."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApprovalDecision {
    Approve,
    Deny,
}

impl ApprovalDecision {
    pub(crate) fn as_effect(self) -> Effect {
        match self {
            ApprovalDecision::Approve => Effect::Allow,
            ApprovalDecision::Deny => Effect::Deny,
        }
    }
}
