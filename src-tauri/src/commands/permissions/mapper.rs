use super::dto::{PendingApprovalEntry, PrincipalEntry, SkillApprovalEntry};
use crate::contexts::permissions::api::{
    ApprovalDecision, ApprovalRequest, PolicyTemplateName, Principal, RiskLevel, Scope,
};

pub(super) fn pending_approval_to_dto(request: ApprovalRequest) -> PendingApprovalEntry {
    PendingApprovalEntry {
        id: request.id,
        agent_id: request.agent_id,
        session_id: request.session_id,
        call_id: request.call_id,
        action: request.action.as_str().to_string(),
        resource: request.resource.as_str().to_string(),
        risk_level: risk_level_str(request.risk_level).to_string(),
        skill: request.skill.map(|skill| SkillApprovalEntry {
            parent_agent_id: skill.parent_agent_id,
            skill_id: skill.skill_id,
            tool_id: skill.tool_id,
            effective_revision: skill.effective_revision,
            source_scope: skill.source_scope,
            requested_capability: skill.requested_capability,
            delegated_operation: skill.delegated_operation,
            redacted_input_summary: skill.redacted_input_summary,
            immutable_witness: skill.immutable_witness,
        }),
        created_at: request.created_at,
    }
}

pub(super) fn principal_to_dto(
    (principal, has_explicit_assignment): (Principal, bool),
) -> PrincipalEntry {
    let template = principal.template();
    PrincipalEntry {
        agent_id: principal.agent_id().to_string(),
        template: template.as_str().to_string(),
        requires_confirmation_to_assign: template.requires_confirmation_to_assign(),
        has_explicit_assignment,
    }
}

pub(super) fn parse_template(value: &str) -> Option<PolicyTemplateName> {
    PolicyTemplateName::from_str(value)
}

pub(super) fn parse_scope(value: &str) -> Scope {
    match value {
        "session" => Scope::Session,
        "project" => Scope::Project,
        "global" => Scope::Global,
        _ => Scope::Once,
    }
}

// The `bool` -> `ToolApprovalDecision` mapping moved to the routed delivery adapter in bootstrap.
// It belonged to whichever layer actually talks to `agent_runtime`, and the command no longer does:
// it hands a decision to one use case and returns that use case's typed outcome.

pub(super) fn approval_decision(approved: bool) -> ApprovalDecision {
    if approved {
        ApprovalDecision::Approve
    } else {
        ApprovalDecision::Deny
    }
}

fn risk_level_str(risk_level: RiskLevel) -> &'static str {
    match risk_level {
        RiskLevel::L0 => "L0",
        RiskLevel::L1 => "L1",
        RiskLevel::L2 => "L2",
        RiskLevel::L3 => "L3",
    }
}
