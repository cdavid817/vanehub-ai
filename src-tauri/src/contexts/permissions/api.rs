//! Published in-process facade for the `permissions` context.
//!
//! Other contexts (`agent_runtime`) and Tauri command adapters use this facade instead of
//! reaching into `permissions`' application services or repositories directly.

use super::application::{ApprovalBroker, EvaluationService};

pub(crate) use super::application::{PermissionsApplicationError, ResolvedApproval};
pub(crate) use super::domain::{
    Action, ApprovalDecision, ApprovalRequest, Effect, Principal, PolicyTemplateName, Resource,
    RiskLevel, Scope,
};

#[derive(Clone)]
pub(crate) struct PermissionsApi {
    evaluation: EvaluationService,
    approvals: ApprovalBroker,
}

impl PermissionsApi {
    pub(crate) fn new(evaluation: EvaluationService, approvals: ApprovalBroker) -> Self {
        Self {
            evaluation,
            approvals,
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
    pub(crate) fn assign_template(
        &self,
        agent_id: &str,
        template: PolicyTemplateName,
    ) -> Result<Principal, PermissionsApplicationError> {
        self.evaluation.assign_template(agent_id, template)
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
}
