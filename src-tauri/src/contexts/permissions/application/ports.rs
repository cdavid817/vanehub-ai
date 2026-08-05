//! Consuming-side I/O contracts for the evaluation engine and `ApprovalBroker`. Narrow and
//! behavior-oriented (project.md's application-port rule) — no raw SQL, no `rusqlite::Connection`,
//! no `tauri::AppHandle` crosses this boundary.

use super::error::PermissionsApplicationError;
use crate::contexts::permissions::domain::{
    Action, Effect, Grant, Principal, PolicyTemplateName, Resource, RiskLevel,
};

pub(crate) trait PermissionsClockPort: Send + Sync {
    fn now(&self) -> String;
}

pub(crate) trait PermissionsIdPort: Send + Sync {
    fn next_id(&self, prefix: &str) -> String;
}

pub(crate) trait PrincipalRepository: Send + Sync {
    fn find_by_agent_id(
        &self,
        agent_id: &str,
    ) -> Result<Option<Principal>, PermissionsApplicationError>;
    fn create(&self, principal: &Principal) -> Result<(), PermissionsApplicationError>;
    fn update_template(
        &self,
        principal_id: &str,
        template: PolicyTemplateName,
    ) -> Result<(), PermissionsApplicationError>;
}

pub(crate) struct GrantQuery<'a> {
    pub(crate) principal_id: &'a str,
    pub(crate) action: &'a Action,
    pub(crate) resource: &'a Resource,
    pub(crate) session_id: &'a str,
    pub(crate) project_key: &'a str,
}

pub(crate) trait GrantRepository: Send + Sync {
    fn find_matching(
        &self,
        query: &GrantQuery<'_>,
    ) -> Result<Option<Grant>, PermissionsApplicationError>;
    fn create(&self, grant: &Grant) -> Result<(), PermissionsApplicationError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuditDecider {
    /// The evaluation engine itself resolved this (a matching grant, a template rule, or the MCP
    /// floor) — no human was involved, whether the outcome was `Allow`, `Deny`, or `Ask`.
    Policy,
    /// A human explicitly approved or denied a pending approval.
    Human,
    /// A pending approval expired unresolved (design.md D5): resolves `Deny`, fail-closed.
    Timeout,
    /// A resolution arrived for a generation that had already ended (design.md D6): rejected,
    /// not applied.
    StaleGeneration,
}

pub(crate) struct AuditRecord {
    pub(crate) id: String,
    pub(crate) principal_id: String,
    pub(crate) session_id: String,
    pub(crate) generation_id: String,
    pub(crate) action: Action,
    pub(crate) resource: Resource,
    pub(crate) effect: Effect,
    pub(crate) risk_level: RiskLevel,
    pub(crate) decider: AuditDecider,
    /// Phase 1 only ever writes `"native_agent"`; reserved for `mcp_callback`/`policy_static`/
    /// `pty_interception` in later phases (design.md Roadmap).
    pub(crate) channel: &'static str,
    pub(crate) created_at: String,
}

pub(crate) trait AuditRepository: Send + Sync {
    fn append(&self, record: AuditRecord) -> Result<(), PermissionsApplicationError>;
}
