//! Maps Skill-delegated native operations onto the existing unified permission decision point.

use crate::contexts::permissions::api::{
    Action, Effect, PermissionsApi, Resource, SkillApprovalProvenance,
};
use crate::contexts::tooling::skill_tools::application::{
    SkillToolApplicationError, SkillToolApprovalPort, SkillToolPermissionDecision,
    SkillToolPermissionPort, SkillToolPrincipal,
};
use crate::contexts::tooling::skill_tools::domain::SkillToolCapability;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;

trait SkillPolicyEvaluation: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    fn evaluate(
        &self,
        agent_id: &str,
        action: Action,
        resource: Resource,
        session_id: &str,
        generation_id: &str,
        project_key: &str,
    ) -> Effect;

    #[allow(clippy::too_many_arguments)]
    fn create_skill_pending(
        &self,
        provenance: SkillApprovalProvenance,
        action: Action,
        resource: Resource,
        session_id: &str,
        generation_id: &str,
        call_id: &str,
        project_key: &str,
    ) -> Result<String, String>;
}

impl SkillPolicyEvaluation for PermissionsApi {
    fn evaluate(
        &self,
        agent_id: &str,
        action: Action,
        resource: Resource,
        session_id: &str,
        generation_id: &str,
        project_key: &str,
    ) -> Effect {
        PermissionsApi::evaluate(
            self,
            agent_id,
            action,
            resource,
            session_id,
            generation_id,
            project_key,
        )
    }

    fn create_skill_pending(
        &self,
        provenance: SkillApprovalProvenance,
        action: Action,
        resource: Resource,
        session_id: &str,
        generation_id: &str,
        call_id: &str,
        project_key: &str,
    ) -> Result<String, String> {
        self.create_skill_pending_approval(
            provenance,
            action,
            resource,
            session_id,
            generation_id,
            call_id,
            project_key,
        )
        .map(|request| request.id)
        .map_err(|error| error.to_string())
    }
}

#[derive(Clone)]
pub(crate) struct SkillToolPermissionAdapter {
    permissions: Arc<dyn SkillPolicyEvaluation>,
    approval_channel_available: bool,
}

impl SkillToolPermissionAdapter {
    #[allow(dead_code)]
    pub(crate) fn new(permissions: PermissionsApi, approval_channel_available: bool) -> Self {
        Self {
            permissions: Arc::new(permissions),
            approval_channel_available,
        }
    }

    #[cfg(test)]
    fn with_evaluator(
        permissions: Arc<dyn SkillPolicyEvaluation>,
        approval_channel_available: bool,
    ) -> Self {
        Self {
            permissions,
            approval_channel_available,
        }
    }
}

impl SkillToolPermissionPort for SkillToolPermissionAdapter {
    fn evaluate(
        &self,
        principal: &SkillToolPrincipal,
        capability: &SkillToolCapability,
        arguments: &Value,
    ) -> SkillToolPermissionDecision {
        let Some(session_id) = principal.session_id.as_deref() else {
            return SkillToolPermissionDecision::Deny;
        };
        let (action, resource) = skill_action_and_resource(capability.operation(), arguments);
        let project_key = principal.workspace_path.as_deref().unwrap_or("global");
        match self.permissions.evaluate(
            &principal.parent_agent_id,
            action,
            resource,
            session_id,
            &principal.generation_id,
            project_key,
        ) {
            Effect::Allow => SkillToolPermissionDecision::Allow,
            Effect::Ask => SkillToolPermissionDecision::Ask,
            Effect::Deny => SkillToolPermissionDecision::Deny,
        }
    }
}

impl SkillToolApprovalPort for SkillToolPermissionAdapter {
    fn create_pending(
        &self,
        principal: &SkillToolPrincipal,
        capability: &SkillToolCapability,
        arguments: &Value,
        call_id: &str,
    ) -> Result<String, SkillToolApplicationError> {
        if !self.approval_channel_available {
            return Err(SkillToolApplicationError::HostDenied(
                "approval-channel-unavailable".to_string(),
            ));
        }
        let session_id = principal
            .session_id
            .as_deref()
            .ok_or_else(|| SkillToolApplicationError::HostDenied("principal-context".into()))?;
        let (action, resource) = skill_action_and_resource(capability.operation(), arguments);
        let provenance = approval_provenance(principal, capability, arguments, &action, &resource);
        self.permissions
            .create_skill_pending(
                provenance,
                action,
                resource,
                session_id,
                &principal.generation_id,
                call_id,
                principal.workspace_path.as_deref().unwrap_or("global"),
            )
            .map_err(SkillToolApplicationError::Storage)
    }
}

fn approval_provenance(
    principal: &SkillToolPrincipal,
    capability: &SkillToolCapability,
    arguments: &Value,
    action: &Action,
    resource: &Resource,
) -> SkillApprovalProvenance {
    let source = &principal.key.source;
    let source_scope = match source.workspace_path.as_deref() {
        Some(path) => format!("workspace:{path}"),
        None => source.scope.as_str().to_string(),
    };
    SkillApprovalProvenance {
        parent_agent_id: principal.parent_agent_id.clone(),
        skill_id: principal.key.owner.as_str().to_string(),
        tool_id: principal.key.tool.as_str().to_string(),
        effective_revision: principal.key.revision.as_str().to_string(),
        source_scope,
        requested_capability: capability.as_declaration(),
        delegated_operation: capability.operation().to_string(),
        redacted_input_summary: redacted_summary(arguments),
        immutable_witness: request_witness(principal, capability, arguments, action, resource),
    }
}

fn redacted_summary(arguments: &Value) -> String {
    let summary = match arguments {
        Value::Object(fields) => Value::Object(
            fields
                .keys()
                .take(16)
                .map(|key| (key.clone(), Value::String("[REDACTED]".to_string())))
                .collect(),
        ),
        _ => Value::String("[REDACTED]".to_string()),
    };
    summary.to_string().chars().take(512).collect()
}

fn request_witness(
    principal: &SkillToolPrincipal,
    capability: &SkillToolCapability,
    arguments: &Value,
    action: &Action,
    resource: &Resource,
) -> String {
    let mut digest = Sha256::new();
    for value in [
        principal.key.revision.as_str(),
        principal.session_id.as_deref().unwrap_or(""),
        &principal.generation_id,
        &capability.as_declaration(),
        action.as_str(),
        resource.as_str(),
    ] {
        digest.update((value.len() as u64).to_be_bytes());
        digest.update(value.as_bytes());
    }
    digest.update(serde_json::to_vec(arguments).unwrap_or_default());
    let encoded: String = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("sha256:{encoded}")
}

fn skill_action_and_resource(operation: &str, input: &Value) -> (Action, Resource) {
    let path = || input.get("path").and_then(Value::as_str).unwrap_or("");
    match operation {
        "read_file" | "find_definition" | "find_references" | "get_hover" | "get_diagnostics" => {
            (Action::file_read(), Resource::file_path(path()))
        }
        "write_file" | "edit" => (Action::file_write(), Resource::file_path(path())),
        "grep" | "glob" | "search_code" => (Action::file_read(), Resource::workspace()),
        "remember" => (Action::memory_write(), Resource::memory()),
        "recall" => (Action::file_read(), Resource::memory()),
        name if name.starts_with("mcp__") => (Action::mcp_tool(), Resource::new(name)),
        name => (
            Action::new(format!("skill-tool:{name}")),
            Resource::new(name),
        ),
    }
}

#[cfg(test)]
#[path = "skill_tool_permission_adapter_tests.rs"]
mod tests;
