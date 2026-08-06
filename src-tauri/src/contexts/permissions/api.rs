//! Published in-process facade for the `permissions` context.
//!
//! Other contexts (`agent_runtime`) and Tauri command adapters use this facade instead of
//! reaching into `permissions`' application services or repositories directly.

use super::application::{ApprovalBroker, ClaudeCodeHookPort, EvaluationService};
use super::infrastructure::HookWaitRegistry;
use std::sync::Arc;

pub(crate) use super::application::{PermissionsApplicationError, ResolvedApproval};
pub(crate) use super::domain::{
    Action, ApprovalDecision, ApprovalRequest, Effect, PolicyTemplateName, Principal, Resource,
    RiskLevel, Scope, CLAUDE_CODE_AGENT_ID,
};

#[derive(Clone)]
pub(crate) struct PermissionsApi {
    evaluation: EvaluationService,
    approvals: ApprovalBroker,
    hook_waits: Arc<HookWaitRegistry>,
    claude_code_hook: Arc<dyn ClaudeCodeHookPort>,
}

impl PermissionsApi {
    pub(crate) fn new(
        evaluation: EvaluationService,
        approvals: ApprovalBroker,
        hook_waits: Arc<HookWaitRegistry>,
        claude_code_hook: Arc<dyn ClaudeCodeHookPort>,
    ) -> Self {
        Self {
            evaluation,
            approvals,
            hook_waits,
            claude_code_hook,
        }
    }

    /// The PDP entry point: resolves `(agent, action, resource)` to an `Effect`. Never fails —
    /// internal errors fail closed to `Ask` (design.md Risks).
    pub(crate) fn evaluate(
        &self,
        agent_id: &str,
        action: Action,
        resource: Resource,
        session_id: &str,
        generation_id: &str,
        project_key: &str,
    ) -> Effect {
        self.evaluation.evaluate(
            agent_id,
            action,
            resource,
            session_id,
            generation_id,
            project_key,
        )
    }

    /// Assigns a policy template to an agent's principal (`permissions-approval`'s template
    /// picker). Confirmation-on-increase is a caller/UI concern, not enforced here.
    ///
    /// For the `claude-code` principal specifically, also (re)installs the permission hook —
    /// unconditionally, on every assignment, not just the first: every template (even
    /// `readonly`) depends on the hook actually being registered for its `evaluate()` outcome to
    /// have any effect on Claude Code at all, and `set_permission_hook_entries` is idempotent
    /// (`add-claude-code-permission-callback` design.md D6/D7). A failure here fails the whole
    /// call rather than leaving the caller believing a template took effect when the mechanism
    /// that would enforce it isn't actually active. The first-use confirmation dialog itself is
    /// a frontend/command-layer concern (`permissions-approval`'s "Enabling Claude Code hook
    /// management requires a distinct first-use confirmation") — this method has no notion of
    /// "first" vs "subsequent," only "claude-code or not."
    pub(crate) fn assign_template(
        &self,
        agent_id: &str,
        template: PolicyTemplateName,
    ) -> Result<Principal, PermissionsApplicationError> {
        let principal = self.evaluation.assign_template(agent_id, template)?;
        if agent_id == CLAUDE_CODE_AGENT_ID {
            self.claude_code_hook.install()?;
        }
        Ok(principal)
    }

    /// Reports an agent's current policy template — synthesizing the effective default when no
    /// principal row exists yet — without ever creating one as a side effect of reading
    /// (`add-permissions-settings-ui`'s agent-policy list). The `bool` is whether that template
    /// comes from a real, previously-assigned row; see `EvaluationService::find_principal`.
    pub(crate) fn find_principal(
        &self,
        agent_id: &str,
    ) -> Result<(Principal, bool), PermissionsApplicationError> {
        self.evaluation.find_principal(agent_id)
    }

    /// Registers a new pending approval — called by a PEP integration after `evaluate` resolves
    /// `Ask`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_pending_approval(
        &self,
        agent_id: &str,
        action: Action,
        resource: Resource,
        session_id: &str,
        generation_id: &str,
        call_id: &str,
        project_key: &str,
    ) -> Result<ApprovalRequest, PermissionsApplicationError> {
        self.approvals.create_pending(
            agent_id,
            action,
            resource,
            session_id,
            generation_id,
            call_id,
            project_key,
        )
    }

    pub(crate) fn list_pending_approvals(&self) -> Vec<ApprovalRequest> {
        self.approvals.list_pending()
    }

    pub(crate) fn get_pending_approval(&self, request_id: &str) -> Option<ApprovalRequest> {
        self.approvals.get_pending(request_id)
    }

    /// Finalizes a pending approval. See `ApprovalBroker::finalize` for what `delivered` means
    /// and why the caller (a `permissions` command handler) determines it.
    pub(crate) fn finalize_pending_approval(
        &self,
        request_id: &str,
        decision: ApprovalDecision,
        scope: Scope,
        delivered: bool,
    ) -> Result<Option<ResolvedApproval>, PermissionsApplicationError> {
        self.approvals
            .finalize(request_id, decision, scope, delivered)
    }

    /// Sweeps every pending approval past its timeout window, resolving each as `Deny`. The
    /// caller is responsible for delivering that denial back to each request's waiting
    /// generation through the same PEP-specific channel it was raised through.
    pub(crate) fn sweep_timed_out_approvals(&self) -> Vec<ApprovalRequest> {
        self.approvals.sweep_timed_out()
    }

    /// Delivers a resolution to the Claude Code hook bridge's own waiting HTTP request, if
    /// `request_id` was raised through that channel (`claude-code-permission-hook`). Returns
    /// `false` harmlessly for a request raised through any other channel, or one already
    /// resolved — callers are expected to try every registered delivery channel unconditionally
    /// and combine the results, not branch on which one applies (`resolve_pending_approval`'s
    /// zero-branching command-adapter rule).
    pub(crate) fn resolve_hook_wait(&self, request_id: &str, effect: Effect) -> bool {
        self.hook_waits.resolve(request_id, effect)
    }
}
