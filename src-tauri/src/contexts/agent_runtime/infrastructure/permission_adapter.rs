//! Adapts `permissions::api::PermissionsApi` (a published cross-context facade) to
//! `AgentPermissionPort`, the native tool-use loop's own dependency-inversion boundary.

use crate::contexts::agent_runtime::application::{AgentPermissionPort, AgentRuntimeApplicationError};
use crate::contexts::permissions::api::PermissionsApi;
use crate::contexts::permissions::domain::{Action, Effect, Resource};

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
