//! Consuming-side I/O contracts for the evaluation engine and `ApprovalBroker`. Narrow and
//! behavior-oriented (project.md's application-port rule) — no raw SQL, no `rusqlite::Connection`,
//! no `tauri::AppHandle` crosses this boundary.

use super::error::PermissionsApplicationError;
use crate::contexts::permissions::domain::{
    Action, ApprovalDecisionRecord, ApprovalRequest, ApprovalResolution, ApprovalResolutionId,
    ApprovalResolutionState, CanonicalGrantKey, Effect, Grant, PersistedEffect, PolicyTemplateName,
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

/// Reading remembered decisions.
///
/// One operation, deliberately. There is no `create` — an append is what let one canonical key hold
/// several disagreeing rows with nothing in the schema to say which of them was the answer — and
/// there is no write at all, because every write to a grant now happens inside the resolution
/// transaction that authorises it. A port method that could write a grant on its own connection
/// would be a way to create authority without a decision behind it.
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
    /// Storage was unavailable and the bounded approval timeout would otherwise have been
    /// violated. Only ever `Deny`, and never turns into an `Allow` on retry.
    EmergencyFailClosed,
    /// Evaluation itself could not complete — a storage failure, not a policy outcome. Recorded
    /// under its own decider so an operator can tell "the rules said Ask" from "the rules could
    /// not be read", which look identical in the effect column.
    EvaluationError,
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
    /// The resolution this audit row belongs to, when it belongs to one. `None` for evaluation
    /// records, which are decisions the engine made without anybody being asked.
    pub(crate) resolution_id: Option<String>,
    /// A stable safe code, never a message or a path. Present when the row records something the
    /// effect column alone cannot express — an evaluation failure, a delivery outcome.
    pub(crate) outcome_reason: Option<&'static str>,
    pub(crate) created_at: String,
}

pub(crate) trait AuditRepository: Send + Sync {
    fn append(&self, record: AuditRecord) -> Result<(), PermissionsApplicationError>;
}

/// The last resort when the audit trail itself cannot be written.
///
/// A port rather than a direct call into unified logging, because the application layer must not
/// know where logs go — and because the thing that usually fails here is storage, which is also
/// what the audit trail is. Infallible by construction: a diagnostic that could fail would need its
/// own fallback, and there is nowhere further to fall.
///
/// Everything it accepts is a bounded token or an id. No resource, no tool input, no error text:
/// the first two are user content and the last can quote a query.
pub(crate) trait PermissionsDiagnosticsPort: Send + Sync {
    fn evaluation_failed_closed(
        &self,
        action: &Action,
        reason: &'static str,
        session_id: &str,
        generation_id: &str,
    );
}

/// The decision as it is about to become durable, before storage assigns it a state history.
pub(crate) struct NewApprovalResolution {
    pub(crate) id: ApprovalResolutionId,
    pub(crate) request_id: String,
    pub(crate) principal_id: String,
    pub(crate) session_id: String,
    pub(crate) generation_id: String,
    /// A bounded correlation hash, never the provider's own call id. The raw value is a
    /// provider-chosen string that can carry request content; the hash is enough to correlate a
    /// delivery with its waiter and carries nothing back out.
    pub(crate) call_id_hash: String,
    pub(crate) action: Action,
    pub(crate) resource: Resource,
    pub(crate) risk_level: RiskLevel,
    pub(crate) decision: ApprovalDecisionRecord,
    pub(crate) state: ApprovalResolutionState,
    pub(crate) now: String,
}

/// Everything one approval resolution makes durable, in one value.
///
/// Passed as a unit rather than as three arguments because the three writes are one consistency
/// boundary: the previous code called `GrantRepository::create` and then `AuditRepository::append`,
/// and a failure between them left an action released with no record of why.
pub(crate) struct ResolutionCommit {
    pub(crate) resolution: NewApprovalResolution,
    pub(crate) audit: AuditRecord,
    /// Absent for `Once`, for a denial nobody asked to remember, and for every stale or
    /// emergency outcome. When present it is written inactive, in the same transaction.
    pub(crate) grant_intent: Option<PendingGrantIntent>,
}

/// The durable ledger of approval decisions and their delivery.
///
/// Owned by `permissions`. Every operation is a whole consistency boundary rather than a row edit,
/// which is what keeps the transaction inside infrastructure instead of leaking a connection to a
/// use case that would then have to know when to commit.
pub(crate) trait ApprovalResolutionRepository: Send + Sync {
    /// Writes the resolution, its audit row, and any grant intent in one transaction. All of them
    /// commit or none of them do.
    fn commit_resolution(
        &self,
        commit: &ResolutionCommit,
    ) -> Result<ApprovalResolution, PermissionsApplicationError>;

    /// The resolution for one approval request, if it has one.
    ///
    /// What makes a retry after an ambiguous response return the existing answer instead of
    /// producing a second decision or an unhelpful not-found.
    fn find_by_request_id(
        &self,
        request_id: &str,
    ) -> Result<Option<ApprovalResolution>, PermissionsApplicationError>;

    /// Records that delivery was attempted and failed. Bumps the attempt counter and stores a
    /// stable code; never changes the decision and never activates a grant.
    fn record_delivery_failure(
        &self,
        id: &ApprovalResolutionId,
        error_code: &str,
        now: &str,
    ) -> Result<ApprovalResolution, PermissionsApplicationError>;

    /// Records the waiter's acknowledgement and activates the grant intent, in one transaction.
    /// Idempotent: a second acknowledgement leaves one delivered resolution and one active grant
    /// revision.
    fn acknowledge_delivery_and_activate(
        &self,
        id: &ApprovalResolutionId,
        now: &str,
    ) -> Result<ApprovalResolution, PermissionsApplicationError>;

    /// Reconciles every resolution that could still have had a live waiter when the process ended.
    /// Returns how many rows were reconciled. Grants stay inactive; nothing is re-delivered.
    fn mark_aborted_by_restart(&self, now: &str) -> Result<usize, PermissionsApplicationError>;
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
