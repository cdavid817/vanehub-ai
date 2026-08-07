//! Expert role management. Roles are reusable assets: a session seat binds one to an Agent, so the
//! same installed CLI can review in one session and architect in another.

use super::AgentRuntimeApplicationError;
use crate::contexts::agent_runtime::domain::{ExpertRole, ExpertRoleInput, ExpertRoleOrigin};
use std::sync::Arc;

pub(crate) trait ExpertRolePort: Send + Sync {
    fn list(&self) -> Result<Vec<ExpertRole>, AgentRuntimeApplicationError>;
    fn upsert(&self, role: &ExpertRole) -> Result<(), AgentRuntimeApplicationError>;
    fn delete(&self, role_id: &str) -> Result<(), AgentRuntimeApplicationError>;
}

pub(crate) trait ExpertRoleClockPort: Send + Sync {
    fn now(&self) -> String;
}

pub(crate) trait ExpertRoleIdPort: Send + Sync {
    fn next_id(&self) -> String;
}

pub(crate) struct ExpertRoleApplicationPorts {
    pub(crate) repository: Arc<dyn ExpertRolePort>,
    pub(crate) clock: Arc<dyn ExpertRoleClockPort>,
    pub(crate) ids: Arc<dyn ExpertRoleIdPort>,
    /// Shipped roles a user can assign without authoring instruction text first. They are never
    /// persisted, so editing one is rejected rather than silently forking it.
    pub(crate) builtins: Vec<ExpertRole>,
}

#[derive(Clone)]
pub(crate) struct ExpertRoleApplicationService {
    repository: Arc<dyn ExpertRolePort>,
    clock: Arc<dyn ExpertRoleClockPort>,
    ids: Arc<dyn ExpertRoleIdPort>,
    builtins: Arc<Vec<ExpertRole>>,
}

impl ExpertRoleApplicationService {
    pub(crate) fn new(ports: ExpertRoleApplicationPorts) -> Self {
        Self {
            repository: ports.repository,
            clock: ports.clock,
            ids: ports.ids,
            builtins: Arc::new(ports.builtins),
        }
    }

    pub(crate) fn list(&self) -> Result<Vec<ExpertRole>, AgentRuntimeApplicationError> {
        let mut roles = self.builtins.as_ref().clone();
        roles.extend(self.repository.list()?);
        Ok(roles)
    }

    pub(crate) fn save(
        &self,
        role_id: Option<String>,
        input: ExpertRoleInput,
    ) -> Result<ExpertRole, AgentRuntimeApplicationError> {
        let now = self.clock.now();
        let existing = match role_id.as_deref() {
            Some(id) => Some(self.require_editable(id)?),
            None => None,
        };
        let id = existing
            .as_ref()
            .map(|role| role.id.clone())
            .unwrap_or_else(|| self.ids.next_id());
        let created_at = existing
            .as_ref()
            .map(|role| role.created_at.clone())
            .unwrap_or_else(|| now.clone());
        let role = ExpertRole::new(id, input, created_at, now)
            .map_err(|error| AgentRuntimeApplicationError::Registry(error.to_string()))?;
        self.repository.upsert(&role)?;
        Ok(role)
    }

    pub(crate) fn delete(&self, role_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.require_editable(role_id)?;
        self.repository.delete(role_id)
    }

    fn require_editable(&self, role_id: &str) -> Result<ExpertRole, AgentRuntimeApplicationError> {
        if let Some(builtin) = self.builtins.iter().find(|role| role.id == role_id) {
            return Err(AgentRuntimeApplicationError::Registry(format!(
                "built-in expert role '{}' cannot be modified",
                builtin.display_name
            )));
        }
        self.repository
            .list()?
            .into_iter()
            .find(|role| role.id == role_id && role.origin == ExpertRoleOrigin::User)
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Registry(format!("expert role not found: {role_id}"))
            })
    }
}
