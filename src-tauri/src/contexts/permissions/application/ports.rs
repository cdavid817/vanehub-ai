//! Consuming-side I/O contracts for the evaluation engine and `ApprovalBroker`. Narrow and
//! behavior-oriented (project.md's application-port rule) — no raw SQL, no `rusqlite::Connection`,
//! no `tauri::AppHandle` crosses this boundary.

use super::error::PermissionsApplicationError;
use crate::contexts::permissions::domain::{
    Action, ApprovalRequest, Effect, Grant, Principal, PolicyTemplateName, Resource, RiskLevel,
};

pub(crate) trait PermissionsClockPort: Send + Sync {
    fn now(&self) -> String;
}

pub(crate) trait PermissionsIdPort: Send + Sync {
    fn next_id(&self, prefix: &str) -> String;
}

/// The user-configurable template a newly created principal is assigned
/// (`permissions-core`'s "Newly created principals default to a configurable template").
/// Infallible by construction: an implementation must resolve any read failure or absent setting
/// to `PolicyTemplateName::Standard` itself rather than surface an error here, so
/// `EvaluationService` never has to decide what "the default is unavailable" means.
pub(crate) trait DefaultTemplatePort: Send + Sync {
    fn default_template(&self) -> PolicyTemplateName;
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

/// Pushes notice of a newly created pending approval (`permissions-approval`'s "New pending
/// approvals are pushed and reconciled by pull") — a best-effort UX signal, never the correctness
/// boundary. The frontend's own mount-time pull against `list_pending` is what actually guarantees
/// a missed event can't leave a generation silently waiting forever, so a failure here is not
/// propagated as an error by callers; there's nothing meaningful to retry.
pub(crate) trait PendingApprovalEventPort: Send + Sync {
    fn publish(&self, request: &ApprovalRequest) -> Result<(), PermissionsApplicationError>;
}
