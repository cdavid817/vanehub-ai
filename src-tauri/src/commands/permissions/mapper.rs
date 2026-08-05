use super::dto::{PendingApprovalEntry, PrincipalEntry};
use crate::contexts::agent_runtime::application::ToolApprovalDecision;
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
        created_at: request.created_at,
    }
}

pub(super) fn principal_to_dto(principal: Principal) -> PrincipalEntry {
    let template = principal.template();
    PrincipalEntry {
        agent_id: principal.agent_id().to_string(),
        template: template.as_str().to_string(),
        requires_confirmation_to_assign: template.requires_confirmation_to_assign(),
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

pub(super) fn tool_approval_decision(approved: bool) -> ToolApprovalDecision {
    if approved {
        ToolApprovalDecision::Approved
    } else {
        ToolApprovalDecision::Denied
    }
}

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
