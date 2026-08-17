//! Emits `permission:request` on the Tauri event bus for every newly created pending approval
//! (design.md D7-adjacent; `permissions-approval`'s "New pending approvals are pushed and
//! reconciled by pull"), mirroring `agent_runtime::infrastructure::events`'s
//! `TauriAgentRuntimeEventAdapter` pattern exactly.

use crate::contexts::permissions::application::PendingApprovalEventPort;
use crate::contexts::permissions::application::PermissionsApplicationError;
use crate::contexts::permissions::domain::ApprovalRequest;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Clone)]
pub(crate) struct TauriPendingApprovalEventAdapter {
    app: AppHandle,
}

impl TauriPendingApprovalEventAdapter {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl PendingApprovalEventPort for TauriPendingApprovalEventAdapter {
    fn publish(&self, request: &ApprovalRequest) -> Result<(), PermissionsApplicationError> {
        self.app
            .emit("permission:request", pending_approval_event(request))
            .map_err(|error| {
                PermissionsApplicationError::infrastructure("tauri-event", error.to_string())
            })
    }
}

/// Deliberately the same shape as `commands::permissions::dto::PendingApprovalEntry` (a separate,
/// independently-defined type — this context's own infrastructure layer must not depend upward
/// on the `commands` layer) so the frontend can treat a pushed event and a `listPendingApprovals`
/// entry identically.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct PendingApprovalEvent {
    id: String,
    agent_id: String,
    session_id: String,
    call_id: String,
    action: String,
    resource: String,
    risk_level: String,
    created_at: String,
}

fn pending_approval_event(request: &ApprovalRequest) -> PendingApprovalEvent {
    PendingApprovalEvent {
        id: request.id.clone(),
        agent_id: request.agent_id.clone(),
        session_id: request.session_id.clone(),
        call_id: request.call_id.clone(),
        action: request.action.as_str().to_string(),
        resource: request.resource.as_str().to_string(),
        risk_level: risk_level_str(request.risk_level),
        created_at: request.created_at.clone(),
    }
}

fn risk_level_str(risk_level: crate::contexts::permissions::domain::RiskLevel) -> String {
    use crate::contexts::permissions::domain::RiskLevel;
    match risk_level {
        RiskLevel::L0 => "L0",
        RiskLevel::L1 => "L1",
        RiskLevel::L2 => "L2",
        RiskLevel::L3 => "L3",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::permissions::domain::{Action, Resource, RiskLevel};

    #[test]
    fn event_payload_uses_camel_case_and_string_action_resource() {
        let request = ApprovalRequest {
            id: "approval-1".to_string(),
            principal_id: "principal-1".to_string(),
            agent_id: "agent-1".to_string(),
            session_id: "session-1".to_string(),
            generation_id: "generation-1".to_string(),
            call_id: "call-1".to_string(),
            project_key: "project-1".to_string(),
            action: Action::shell_exec(),
            resource: Resource::workspace(),
            risk_level: RiskLevel::L2,
            skill: None,
            created_at: "2026-08-05T00:00:00Z".to_string(),
        };
        let value = serde_json::to_value(pending_approval_event(&request)).expect("serialize");
        assert_eq!(value["id"], "approval-1");
        assert_eq!(value["agentId"], "agent-1");
        assert_eq!(value["sessionId"], "session-1");
        assert_eq!(value["callId"], "call-1");
        assert_eq!(value["action"], "shell.exec");
        assert_eq!(value["resource"], "workspace");
        assert_eq!(value["riskLevel"], "L2");
        assert_eq!(value["createdAt"], "2026-08-05T00:00:00Z");
        assert!(value.get("agent_id").is_none());
    }
}
