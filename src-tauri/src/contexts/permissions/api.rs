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
    RiskLevel, Scope, SkillApprovalInvalidation, SkillApprovalProvenance, CLAUDE_CODE_AGENT_ID,
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

    /// Re-projects the permission-hook entries at desktop startup, iff hook management was
    /// previously enabled. The only durable representation of "enabled" is a real
    /// (non-synthesized) `claude-code` principal row — `assign_template` is what creates it at
    /// the moment it installs the hook — so that row's existence is the trigger. Re-running
    /// `install()` refreshes the wrapper path the global settings entry names, which otherwise
    /// keeps pointing at a pre-update install location forever
    /// (`add-permission-hook-recovery`'s "Hook registration reconverges at desktop startup").
    /// Returns whether a re-projection was attempted; the caller treats failure as best-effort.
    pub(crate) fn reconverge_claude_code_hook(&self) -> Result<bool, PermissionsApplicationError> {
        let (_, assigned) = self.find_principal(CLAUDE_CODE_AGENT_ID)?;
        if !assigned {
            return Ok(false);
        }
        self.claude_code_hook.install()?;
        Ok(true)
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_skill_pending_approval(
        &self,
        provenance: SkillApprovalProvenance,
        action: Action,
        resource: Resource,
        session_id: &str,
        generation_id: &str,
        call_id: &str,
        project_key: &str,
    ) -> Result<ApprovalRequest, PermissionsApplicationError> {
        self.approvals.create_skill_pending(
            provenance,
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

    #[allow(dead_code)]
    pub(crate) fn invalidate_skill_pending_approval(
        &self,
        request_id: &str,
        current_witness: &str,
        reason: SkillApprovalInvalidation,
    ) -> Option<ApprovalRequest> {
        self.approvals
            .invalidate_skill_pending(request_id, current_witness, reason)
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

#[cfg(test)]
pub(crate) fn test_permissions_api(default_template: PolicyTemplateName) -> PermissionsApi {
    struct NoopHook;
    impl ClaudeCodeHookPort for NoopHook {
        fn install(&self) -> Result<(), PermissionsApplicationError> {
            Ok(())
        }

        fn remove(&self) -> Result<(), PermissionsApplicationError> {
            Ok(())
        }
    }

    test_permissions_api_with_hook(default_template, Arc::new(NoopHook))
}

#[cfg(test)]
pub(crate) fn test_permissions_api_with_hook(
    default_template: PolicyTemplateName,
    claude_code_hook: Arc<dyn ClaudeCodeHookPort>,
) -> PermissionsApi {
    use super::application::{DefaultTemplatePort, PendingApprovalEventPort};
    use super::infrastructure::{
        PermissionsSystemClock, PermissionsUuidIdGenerator, SqliteAuditRepository,
        SqliteGrantRepository, SqlitePrincipalRepository,
    };
    use crate::platform::database::NativeDatabase;
    use crate::test_support::TempDirectory;

    struct FixedDefault(PolicyTemplateName);
    impl DefaultTemplatePort for FixedDefault {
        fn default_template(&self) -> PolicyTemplateName {
            self.0
        }
    }

    struct NoopEvents;
    impl PendingApprovalEventPort for NoopEvents {
        fn publish(&self, _request: &ApprovalRequest) -> Result<(), PermissionsApplicationError> {
            Ok(())
        }
    }

    let directory = TempDirectory::new("published-permissions-api");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let principals = Arc::new(SqlitePrincipalRepository::new(database.clone()));
    let grants = Arc::new(SqliteGrantRepository::new(database.clone()));
    let audit = Arc::new(SqliteAuditRepository::new(database));
    let clock = Arc::new(PermissionsSystemClock);
    let ids = Arc::new(PermissionsUuidIdGenerator);
    let evaluation = EvaluationService::new(
        principals.clone(),
        grants.clone(),
        audit.clone(),
        clock.clone(),
        ids.clone(),
        Arc::new(FixedDefault(default_template)),
    );
    let approvals = ApprovalBroker::new(
        principals,
        grants,
        audit,
        clock,
        ids,
        Arc::new(NoopEvents),
        300,
    );
    PermissionsApi::new(
        evaluation,
        approvals,
        Arc::new(HookWaitRegistry::new()),
        claude_code_hook,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    #[derive(Default)]
    struct RecordingHook {
        installs: AtomicUsize,
        fail: AtomicBool,
    }

    impl ClaudeCodeHookPort for RecordingHook {
        fn install(&self) -> Result<(), PermissionsApplicationError> {
            self.installs.fetch_add(1, Ordering::SeqCst);
            if self.fail.load(Ordering::SeqCst) {
                return Err(PermissionsApplicationError::infrastructure(
                    "cli_config",
                    "install failed".to_string(),
                ));
            }
            Ok(())
        }

        fn remove(&self) -> Result<(), PermissionsApplicationError> {
            Ok(())
        }
    }

    #[test]
    fn reconverge_does_nothing_without_an_assigned_claude_code_row() {
        let hook = Arc::new(RecordingHook::default());
        let api = test_permissions_api_with_hook(PolicyTemplateName::Standard, hook.clone());

        let attempted = api
            .reconverge_claude_code_hook()
            .expect("reconverge should succeed");

        assert!(!attempted);
        assert_eq!(hook.installs.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn reconverge_reinstalls_when_hook_management_was_previously_enabled() {
        let hook = Arc::new(RecordingHook::default());
        let api = test_permissions_api_with_hook(PolicyTemplateName::Standard, hook.clone());
        api.assign_template(CLAUDE_CODE_AGENT_ID, PolicyTemplateName::Readonly)
            .expect("assign should succeed");
        assert_eq!(hook.installs.load(Ordering::SeqCst), 1);

        let attempted = api
            .reconverge_claude_code_hook()
            .expect("reconverge should succeed");

        assert!(attempted);
        assert_eq!(hook.installs.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn reconverge_surfaces_an_install_failure_to_the_best_effort_caller() {
        let hook = Arc::new(RecordingHook::default());
        let api = test_permissions_api_with_hook(PolicyTemplateName::Standard, hook.clone());
        api.assign_template(CLAUDE_CODE_AGENT_ID, PolicyTemplateName::Readonly)
            .expect("assign should succeed");

        hook.fail.store(true, Ordering::SeqCst);
        assert!(api.reconverge_claude_code_hook().is_err());
    }
}
