mod action;
mod approval_request;
mod canonical_grant_key;
mod effect;
mod error;
mod grant;
mod policy;
mod principal;
mod resource;
mod risk_level;
mod scope;
mod template;

/// The stable principal id every Claude Code CLI hook evaluation is attributed to (design.md D2:
/// one global identity for the machine, not scoped per project or session) —
/// `claude-code-permission-hook`'s loopback bridge and `permissions-approval`'s Agent Policies
/// listing both key off this same constant.
pub(crate) const CLAUDE_CODE_AGENT_ID: &str = "claude-code";

pub(crate) use action::Action;
pub(crate) use approval_request::{
    ApprovalDecision, ApprovalRequest, SkillApprovalInvalidation, SkillApprovalProvenance,
};
pub(crate) use canonical_grant_key::{
    CanonicalGrantKey, GrantActivationState, PersistedEffect, RememberedScope,
};
pub(crate) use effect::Effect;
pub(crate) use error::PermissionsDomainError;
pub(crate) use grant::Grant;
pub(crate) use policy::resolve_for;
pub(crate) use principal::Principal;
pub(crate) use resource::Resource;
pub(crate) use risk_level::{risk_level_for, RiskLevel};
pub(crate) use scope::Scope;
pub(crate) use template::{policies_for_template, PolicyTemplateName};
