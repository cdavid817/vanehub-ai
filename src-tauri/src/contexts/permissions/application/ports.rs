//! Consuming-side I/O contracts for the evaluation engine and `ApprovalBroker`. Narrow and
//! behavior-oriented (project.md's application-port rule) — no raw SQL, no `rusqlite::Connection`,
//! no `tauri::AppHandle` crosses this boundary.

use super::error::PermissionsApplicationError;
use crate::contexts::permissions::domain::{
    Action, ApprovalRequest, CanonicalGrantKey, Effect, Grant, PersistedEffect, PolicyTemplateName,
    Principal, Resource, RiskLevel,
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
    /// Reads without creating. Kept separate from `get_or_create` because the settings list has to
    /// be able to ask what an agent's template is without that question assigning it one.
    fn find_by_agent_id(
        &self,
        agent_id: &str,
    ) -> Result<Option<Principal>, PermissionsApplicationError>;

    /// Resolves the principal for a stable agent id, creating it on first use.
    ///
    /// One operation rather than the caller's read-then-write, because those two steps are not
    /// atomic and the `agent_id` index is unique. Two generations starting together would have one
    /// of them lose the insert and surface a storage error, which evaluation then fails closed on —
    /// a first-use `Ask` produced by a race rather than by policy. `id_hint` is used only if this
    /// call is the one that inserts.
    fn get_or_create(
        &self,
        agent_id: &str,
        id_hint: &str,
        default_template: PolicyTemplateName,
    ) -> Result<Principal, PermissionsApplicationError>;

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

/// What a resolution asks storage to remember, before anyone has been told the decision.
///
/// Named "intent" rather than "grant" because that is what it is until delivery is acknowledged.
/// It carries the resolution that produced it so activation can be addressed by that resolution
/// alone: the acknowledgement arrives from the delivery adapter, which knows the resolution id and
/// nothing about grant rows.
pub(crate) struct PendingGrantIntent {
    pub(crate) id: String,
    pub(crate) key: CanonicalGrantKey,
    pub(crate) effect: PersistedEffect,
    pub(crate) resolution_id: String,
    pub(crate) now: String,
}

/// Storage for remembered decisions.
///
/// Behaviour-oriented rather than row-oriented, and the three operations are the whole vocabulary:
/// read the one grant that governs an evaluation, write the one value a canonical key currently
/// has, and turn an acknowledged resolution's intent into something evaluation can see. There is
/// deliberately no `create`. An append is what let one canonical key hold several disagreeing rows
/// with nothing in the schema to say which of them was the answer.
pub(crate) trait GrantRepository: Send + Sync {
    /// The single active grant that governs this evaluation, chosen by scope specificity —
    /// exact Session, then exact Project, then Global.
    ///
    /// The ordering is the implementation's obligation, not the caller's: a repository that
    /// returned an unordered candidate set would put the security rule back into whichever caller
    /// happened to iterate it.
    fn find_effective_grant(
        &self,
        query: &GrantQuery<'_>,
    ) -> Result<Option<Grant>, PermissionsApplicationError>;

    /// Writes the value of one canonical key as `pending_delivery`, replacing whatever that key
    /// held and advancing its revision. Idempotent per resolution: re-running the same intent
    /// leaves one row.
    fn upsert_pending_grant_intent(
        &self,
        intent: &PendingGrantIntent,
    ) -> Result<Grant, PermissionsApplicationError>;

    /// Makes every grant intent recorded for `resolution_id` visible to evaluation. Repeating this
    /// is a no-op rather than a second revision, because a delivery acknowledgement can arrive
    /// more than once and each of them means the same thing.
    fn activate_grant_for_resolution(
        &self,
        resolution_id: &str,
        now: &str,
    ) -> Result<(), PermissionsApplicationError>;
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

/// Bridges to `cli_config`'s independent hook-projection operation
/// (`add-claude-code-permission-callback` design.md D7): installs or removes VaneHub's own
/// `PreToolUse` entries in Claude Code's global `settings.json`. The implementation constructs
/// the actual hook-entry JSON itself (matcher list, wrapper binary path) — `cli_config` stays
/// agnostic to what these entries contain, per that design's own cross-context split.
pub(crate) trait ClaudeCodeHookPort: Send + Sync {
    fn install(&self) -> Result<(), PermissionsApplicationError>;
    /// Implemented and unit-tested on every adapter, but no production flow calls it yet — no
    /// uninstall/policy-switch-away-from-claude-code action exists to invoke it.
    #[allow(dead_code)]
    fn remove(&self) -> Result<(), PermissionsApplicationError>;
}
