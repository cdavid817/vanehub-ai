use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolvePendingApprovalInput {
    pub(crate) request_id: String,
    pub(crate) approved: bool,
    /// `"once"` | `"session"` | `"project"` | `"global"` — unrecognized values fall back to
    /// `"once"` (never remembered), matching the domain `Scope` type's own fail-closed default.
    pub(crate) scope: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApplyPolicyTemplateInput {
    pub(crate) agent_id: String,
    /// `"readonly"` | `"standard"` | `"trusted"` | `"yolo"`.
    pub(crate) template: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetAgentPolicyPrincipalInput {
    pub(crate) agent_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PendingApprovalEntry {
    pub(crate) id: String,
    pub(crate) agent_id: String,
    pub(crate) session_id: String,
    /// Correlates this permissions-side request back to the chat transcript's own tool-use
    /// block id, so the approval card rendered under a specific tool call can find its matching
    /// pending-approval `id` (the value `resolvePendingApproval` actually takes).
    pub(crate) call_id: String,
    pub(crate) action: String,
    pub(crate) resource: String,
    pub(crate) risk_level: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) skill: Option<SkillApprovalEntry>,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillApprovalEntry {
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrincipalEntry {
    pub(crate) agent_id: String,
    pub(crate) template: String,
    /// Whether assigning this principal's *current* template required the confirm-to-increase
    /// step — surfaced so the settings UI can show why (`permissions-approval`'s "Increasing a
    /// principal's trust requires explicit confirmation").
    pub(crate) requires_confirmation_to_assign: bool,
    /// Whether this principal has ever been explicitly assigned a template, versus `template`
    /// being a synthesized effective default. The Agent Policies UI uses this for the
    /// `claude-code` principal specifically, to decide whether to show the first-use
    /// hook-installation confirmation (`add-claude-code-permission-callback`'s "Enabling Claude
    /// Code hook management requires a distinct first-use confirmation").
    pub(crate) has_explicit_assignment: bool,
}
