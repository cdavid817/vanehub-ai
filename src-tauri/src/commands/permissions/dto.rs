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
    pub(crate) created_at: String,
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
}
