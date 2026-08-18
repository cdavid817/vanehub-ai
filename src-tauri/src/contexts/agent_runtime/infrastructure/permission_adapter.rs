//! Adapts `permissions::api::PermissionsApi` (a published cross-context facade) to
//! `AgentPermissionPort`, the native tool-use loop's own dependency-inversion boundary.

use crate::contexts::agent_runtime::application::{
    AgentPermissionPort, AgentRuntimeApplicationError, RunnerError, RunnerErrorKind, RunnerKind,
    RunnerPermissionContext, RunnerPermissionPort, RunnerPolicyWitness,
};
use crate::contexts::permissions::api::PermissionsApi;
use crate::contexts::permissions::domain::{Action, Effect, Resource};
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub(crate) struct PermissionsPortAdapter {
    api: PermissionsApi,
}

impl PermissionsPortAdapter {
    pub(crate) fn new(api: PermissionsApi) -> Self {
        Self { api }
    }
}

impl AgentPermissionPort for PermissionsPortAdapter {
    fn evaluate(
        &self,
        agent_id: &str,
        action: Action,
        resource: Resource,
        session_id: &str,
        generation_id: &str,
        project_key: &str,
    ) -> Effect {
        self.api.evaluate(
            agent_id,
            action,
            resource,
            session_id,
            generation_id,
            project_key,
        )
    }

    fn create_pending_approval(
        &self,
        agent_id: &str,
        action: Action,
        resource: Resource,
        session_id: &str,
        generation_id: &str,
        call_id: &str,
        project_key: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.api
            .create_pending_approval(
                agent_id,
                action,
                resource,
                session_id,
                generation_id,
                call_id,
                project_key,
            )
            .map(|_| ())
            .map_err(|error| AgentRuntimeApplicationError::Permission(error.to_string()))
    }
}

impl RunnerPermissionPort for PermissionsPortAdapter {
    fn authorize(
        &self,
        context: &RunnerPermissionContext,
    ) -> Result<RunnerPolicyWitness, RunnerError> {
        context.selection.validate()?;
        let resource = runner_resource(context)?;
        let effect = self.api.evaluate(
            &context.agent_id,
            Action::new(context.action.clone()),
            Resource::new(resource.clone()),
            &context.session_id,
            &context.generation_id,
            &context.project_key,
        );
        let (principal, _) = self
            .api
            .find_principal(&context.agent_id)
            .map_err(|_| RunnerError::new(RunnerErrorKind::PermissionDenied))?;

        // Local execution existed before Runner selection and remains admitted by default; its
        // provider still enforces the selected tool permission mode. Crossing onto SSH is a new
        // authority boundary and therefore requires an explicit policy Allow.
        if context.selection.kind != RunnerKind::Local && effect != Effect::Allow {
            return Err(RunnerError::new(RunnerErrorKind::PermissionDenied));
        }
        Ok(RunnerPolicyWitness {
            fingerprint: policy_fingerprint(
                context,
                &resource,
                principal.template().as_str(),
                effect,
            ),
        })
    }

    fn revalidate(
        &self,
        context: &RunnerPermissionContext,
        witness: &RunnerPolicyWitness,
    ) -> Result<(), RunnerError> {
        let current = self
            .authorize(context)
            .map_err(|_| RunnerError::new(RunnerErrorKind::AuthorityStale))?;
        if current == *witness {
            Ok(())
        } else {
            Err(RunnerError::new(RunnerErrorKind::AuthorityStale))
        }
    }
}

#[cfg(test)]
#[path = "permission_adapter_tests.rs"]
mod tests;

fn runner_resource(context: &RunnerPermissionContext) -> Result<String, RunnerError> {
    let target = context.selection.target_id.as_deref().unwrap_or("local");
    let revision = context
        .selection
        .target_revision
        .map_or_else(|| "none".to_string(), |value| value.to_string());
    if context.action != "shell.exec" {
        return Err(RunnerError::new(RunnerErrorKind::PermissionDenied));
    }
    Ok(format!(
        "runner/{}/{target}/{revision}",
        context.selection.kind.as_str()
    ))
}

fn policy_fingerprint(
    context: &RunnerPermissionContext,
    resource: &str,
    template: &str,
    effect: Effect,
) -> String {
    let material = format!(
        "v1\0{}\0{}\0{}\0{}\0{}\0{}\0{template}\0{effect:?}",
        context.agent_id,
        context.session_id,
        context.generation_id,
        context.project_key,
        context.action,
        resource,
    );
    format!("sha256:{}", hex_digest(material.as_bytes()))
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
