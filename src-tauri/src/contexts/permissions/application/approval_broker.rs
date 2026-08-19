//! Owns the pending-approval queue as the Rust-side single source of truth (design.md D7).
//! Deliberately in-memory, not SQLite-backed: a pending approval only means anything while its
//! originating generation's process is alive, so there is nothing meaningful to recover across an
//! app restart — matching how `RuntimeAgentApiAdapter`'s own per-generation `pending_approvals`
//! already works today.

use super::error::PermissionsApplicationError;
use super::ports::{
    AuditDecider, AuditRecord, AuditRepository, GrantRepository, PendingApprovalEventPort,
    PermissionsClockPort, PermissionsIdPort, PrincipalRepository,
};
use crate::contexts::permissions::domain::{
    risk_level_for, Action, ApprovalDecision, ApprovalRequest, Effect, Grant, PolicyTemplateName,
    Principal, Resource, Scope, SkillApprovalInvalidation, SkillApprovalProvenance,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone)]
pub(crate) struct ApprovalBroker {
    principals: Arc<dyn PrincipalRepository>,
    grants: Arc<dyn GrantRepository>,
    audit: Arc<dyn AuditRepository>,
    clock: Arc<dyn PermissionsClockPort>,
    ids: Arc<dyn PermissionsIdPort>,
    events: Arc<dyn PendingApprovalEventPort>,
    pending: Arc<Mutex<HashMap<String, ApprovalRequest>>>,
    timeout_seconds: i64,
}

/// The result of resolving (or attempting to resolve) a pending approval.
///
/// `request`/`effect` are read by this module's own tests; no production caller consumes the
/// resolved value beyond `is_some()` today — reserved for a future audit/UI surface.
#[allow(dead_code)]
pub(crate) struct ResolvedApproval {
    pub(crate) request: ApprovalRequest,
    pub(crate) effect: Effect,
}

impl ApprovalBroker {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        principals: Arc<dyn PrincipalRepository>,
        grants: Arc<dyn GrantRepository>,
        audit: Arc<dyn AuditRepository>,
        clock: Arc<dyn PermissionsClockPort>,
        ids: Arc<dyn PermissionsIdPort>,
        events: Arc<dyn PendingApprovalEventPort>,
        timeout_seconds: i64,
    ) -> Self {
        Self {
            principals,
            grants,
            audit,
            clock,
            ids,
            events,
            pending: Arc::new(Mutex::new(HashMap::new())),
            timeout_seconds,
        }
    }

    /// Takes the pending-approval lock, recovering the guard rather than aborting if it is
    /// poisoned. Poisoning means some *other* thread already panicked while holding this lock;
    /// killing this one too turns one failure into two, and it would take the approval queue —
    /// which the whole permissions flow blocks on — down with it. The guarded map is only ever
    /// touched by `insert`/`remove`/`get`, none of which can leave it half-updated, so the
    /// recovered map is structurally sound. Same recovery this repository already uses in
    /// `retrieval/api.rs`, `skill_tools/application/registry.rs` and `platform/network/proxy.rs`.
    fn lock_pending(&self) -> MutexGuard<'_, HashMap<String, ApprovalRequest>> {
        self.pending.lock().unwrap_or_else(|poisoned| {
            debug_assert!(false, "pending approvals mutex poisoned");
            poisoned.into_inner()
        })
    }

    /// Registers a new pending approval — called by a PEP integration (Group 6: the native
    /// agent's tool-use loop) after `EvaluationService::evaluate` resolves `Ask`. `call_id`
    /// correlates back to whatever wait mechanism that integration uses to block until resolved;
    /// `permissions` treats it as an opaque string.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_pending(
        &self,
        agent_id: &str,
        action: Action,
        resource: Resource,
        session_id: &str,
        generation_id: &str,
        call_id: &str,
        project_key: &str,
    ) -> Result<ApprovalRequest, PermissionsApplicationError> {
        self.create_pending_inner(
            agent_id,
            action,
            resource,
            session_id,
            generation_id,
            call_id,
            project_key,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_skill_pending(
        &self,
        provenance: SkillApprovalProvenance,
        action: Action,
        resource: Resource,
        session_id: &str,
        generation_id: &str,
        call_id: &str,
        project_key: &str,
    ) -> Result<ApprovalRequest, PermissionsApplicationError> {
        let agent_id = provenance.parent_agent_id.clone();
        self.create_pending_inner(
            &agent_id,
            action,
            resource,
            session_id,
            generation_id,
            call_id,
            project_key,
            Some(provenance),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn create_pending_inner(
        &self,
        agent_id: &str,
        action: Action,
        resource: Resource,
        session_id: &str,
        generation_id: &str,
        call_id: &str,
        project_key: &str,
        skill: Option<SkillApprovalProvenance>,
    ) -> Result<ApprovalRequest, PermissionsApplicationError> {
        let principal = self.get_or_create_principal(agent_id)?;
        let request = ApprovalRequest {
            id: self.ids.next_id("approval"),
            principal_id: principal.id().to_string(),
            agent_id: agent_id.to_string(),
            session_id: session_id.to_string(),
            generation_id: generation_id.to_string(),
            call_id: call_id.to_string(),
            project_key: project_key.to_string(),
            risk_level: risk_level_for(&action),
            action,
            resource,
            skill,
            created_at: self.clock.now(),
        };
        self.lock_pending()
            .insert(request.id.clone(), request.clone());
        // Best-effort (see `PendingApprovalEventPort`'s own doc comment): a publish failure must
        // not fail approval creation itself, since the frontend's pull-on-mount already covers
        // a missed event.
        let _ = self.events.publish(&request);
        Ok(request)
    }

    /// The full pending list — `permissions-approval`'s "Pending approval state is Rust-side
    /// authoritative" and its pull-reconciliation-on-mount requirement both read this.
    pub(crate) fn list_pending(&self) -> Vec<ApprovalRequest> {
        self.lock_pending().values().cloned().collect()
    }

    pub(crate) fn get_pending(&self, request_id: &str) -> Option<ApprovalRequest> {
        self.lock_pending().get(request_id).cloned()
    }

    pub(crate) fn invalidate_skill_pending(
        &self,
        request_id: &str,
        current_witness: &str,
        reason: SkillApprovalInvalidation,
    ) -> Option<ApprovalRequest> {
        let mut pending = self.lock_pending();
        let request = pending.get(request_id)?;
        let skill = request.skill.as_ref()?;
        let witness_matches = skill.immutable_witness == current_witness;
        let invalid = match reason {
            SkillApprovalInvalidation::WitnessMismatch => !witness_matches,
            SkillApprovalInvalidation::Cancellation
            | SkillApprovalInvalidation::RevisionReplaced
            | SkillApprovalInvalidation::Disabled
            | SkillApprovalInvalidation::Quarantined => witness_matches,
        };
        invalid.then(|| pending.remove(request_id)).flatten()
    }

    /// Finalizes a pending approval. `delivered` distinguishes two outcomes the caller (the
    /// `permissions` command handler, per design.md D8's refinement) determines by first calling
    /// `AgentRuntimeApi::resolve_tool_approval`: `true` means a live generation was actually
    /// unblocked (this was a genuine human decision — `AuditDecider::Human`, and a grant is
    /// created if `scope` is remembered); `false` means the generation had already ended (nothing
    /// to unblock — `AuditDecider::StaleGeneration`, no grant, matching design.md D6). Returns
    /// `None` if `request_id` names no pending approval (already resolved, or never existed).
    pub(crate) fn finalize(
        &self,
        request_id: &str,
        decision: ApprovalDecision,
        scope: Scope,
        delivered: bool,
    ) -> Result<Option<ResolvedApproval>, PermissionsApplicationError> {
        let request = {
            let mut pending = self.lock_pending();
            match pending.remove(request_id) {
                Some(request) => request,
                None => return Ok(None),
            }
        };
        let effect = decision.as_effect();
        let decider = if delivered {
            AuditDecider::Human
        } else {
            AuditDecider::StaleGeneration
        };
        let scope = if request.action.as_str() == "delegation.apply" || request.skill.is_some() {
            Scope::Once
        } else {
            scope
        };
        if delivered && scope.is_remembered() {
            self.grants.create(&Grant {
                id: self.ids.next_id("grant"),
                principal_id: request.principal_id.clone(),
                action: request.action.clone(),
                resource: request.resource.clone(),
                effect,
                scope,
                session_id: matches!(scope, Scope::Session).then(|| request.session_id.clone()),
                project_key: matches!(scope, Scope::Project).then(|| request.project_key.clone()),
                created_at: self.clock.now(),
            })?;
        }
        self.audit.append(AuditRecord {
            id: self.ids.next_id("audit"),
            principal_id: request.principal_id.clone(),
            session_id: request.session_id.clone(),
            generation_id: request.generation_id.clone(),
            action: request.action.clone(),
            resource: request.resource.clone(),
            effect,
            risk_level: request.risk_level,
            decider,
            channel: "native_agent",
            created_at: self.clock.now(),
        })?;
        Ok(Some(ResolvedApproval { request, effect }))
    }

    /// Sweeps every pending approval that has waited longer than the timeout window, resolving
    /// each as a fail-closed `Deny` (design.md D5) — the caller is responsible for delivering
    /// that denial back to the waiting generation via the same PEP-specific channel
    /// `create_pending` was raised through, exactly as a human `Deny` would be.
    pub(crate) fn sweep_timed_out(&self) -> Vec<ApprovalRequest> {
        let now: i64 = self.clock.now().parse().unwrap_or(0);
        let expired: Vec<ApprovalRequest> = {
            let mut pending = self.lock_pending();
            let expired_ids: Vec<String> = pending
                .values()
                .filter(|request| {
                    let created_at: i64 = request.created_at.parse().unwrap_or(now);
                    now.saturating_sub(created_at) >= self.timeout_seconds
                })
                .map(|request| request.id.clone())
                .collect();
            expired_ids
                .into_iter()
                .filter_map(|id| pending.remove(&id))
                .collect()
        };
        // Best-effort: an audit-write failure must not stop the swept request from still being
        // returned, since the caller still needs to unblock its waiting generation regardless
        // (design.md D5 — timeout must always fail closed, never hang).
        for request in &expired {
            let _ = self.audit.append(AuditRecord {
                id: self.ids.next_id("audit"),
                principal_id: request.principal_id.clone(),
                session_id: request.session_id.clone(),
                generation_id: request.generation_id.clone(),
                action: request.action.clone(),
                resource: request.resource.clone(),
                effect: Effect::Deny,
                risk_level: request.risk_level,
                decider: AuditDecider::Timeout,
                channel: "native_agent",
                created_at: self.clock.now(),
            });
        }
        expired
    }

    fn get_or_create_principal(
        &self,
        agent_id: &str,
    ) -> Result<Principal, PermissionsApplicationError> {
        if let Some(principal) = self.principals.find_by_agent_id(agent_id)? {
            return Ok(principal);
        }
        let id = self.ids.next_id("principal");
        let principal = Principal::new(
            id,
            agent_id.to_string(),
            PolicyTemplateName::Standard,
            None,
            None,
        )?;
        self.principals.create(&principal)?;
        Ok(principal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::permissions::domain::RiskLevel;
    use std::sync::Mutex as StdMutex;

    #[derive(Default)]
    struct FakePrincipals(StdMutex<HashMap<String, Principal>>);
    impl PrincipalRepository for FakePrincipals {
        fn find_by_agent_id(
            &self,
            agent_id: &str,
        ) -> Result<Option<Principal>, PermissionsApplicationError> {
            Ok(self.0.lock().unwrap().get(agent_id).cloned())
        }
        fn create(&self, principal: &Principal) -> Result<(), PermissionsApplicationError> {
            self.0
                .lock()
                .unwrap()
                .insert(principal.agent_id().to_string(), principal.clone());
            Ok(())
        }
        fn update_template(
            &self,
            _principal_id: &str,
            _template: PolicyTemplateName,
        ) -> Result<(), PermissionsApplicationError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeGrants(StdMutex<Vec<Grant>>);
    impl GrantRepository for FakeGrants {
        fn find_matching(
            &self,
            _query: &super::super::ports::GrantQuery<'_>,
        ) -> Result<Option<Grant>, PermissionsApplicationError> {
            Ok(None)
        }
        fn create(&self, grant: &Grant) -> Result<(), PermissionsApplicationError> {
            self.0.lock().unwrap().push(grant.clone());
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeAudit(StdMutex<Vec<AuditDecider>>);
    impl AuditRepository for FakeAudit {
        fn append(&self, record: AuditRecord) -> Result<(), PermissionsApplicationError> {
            self.0.lock().unwrap().push(record.decider);
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeEvents(StdMutex<Vec<String>>);
    impl PendingApprovalEventPort for FakeEvents {
        fn publish(&self, request: &ApprovalRequest) -> Result<(), PermissionsApplicationError> {
            self.0.lock().unwrap().push(request.id.clone());
            Ok(())
        }
    }

    struct StepClock(StdMutex<i64>);
    impl PermissionsClockPort for StepClock {
        fn now(&self) -> String {
            let mut value = self.0.lock().unwrap();
            let current = *value;
            *value += 1;
            current.to_string()
        }
    }

    struct FakeIds(StdMutex<u64>);
    impl PermissionsIdPort for FakeIds {
        fn next_id(&self, prefix: &str) -> String {
            let mut counter = self.0.lock().unwrap();
            *counter += 1;
            format!("{prefix}-{counter}")
        }
    }

    fn broker(
        timeout_seconds: i64,
    ) -> (
        ApprovalBroker,
        Arc<FakeGrants>,
        Arc<FakeAudit>,
        Arc<FakeEvents>,
    ) {
        let grants = Arc::new(FakeGrants::default());
        let audit = Arc::new(FakeAudit::default());
        let events = Arc::new(FakeEvents::default());
        let broker = ApprovalBroker::new(
            Arc::new(FakePrincipals::default()),
            grants.clone(),
            audit.clone(),
            Arc::new(StepClock(StdMutex::new(0))),
            Arc::new(FakeIds(StdMutex::new(0))),
            events.clone(),
            timeout_seconds,
        );
        (broker, grants, audit, events)
    }

    #[test]
    fn create_pending_appears_in_list_pending() {
        let (broker, _grants, _audit, _events) = broker(60);
        let request = broker
            .create_pending(
                "agent-1",
                Action::shell_exec(),
                Resource::workspace(),
                "session-1",
                "generation-1",
                "call-1",
                "project-1",
            )
            .unwrap();
        assert_eq!(request.risk_level, RiskLevel::L2);
        assert_eq!(broker.list_pending().len(), 1);
    }

    #[test]
    fn create_pending_publishes_an_event() {
        let (broker, _grants, _audit, events) = broker(60);
        let request = broker
            .create_pending(
                "agent-1",
                Action::shell_exec(),
                Resource::workspace(),
                "session-1",
                "generation-1",
                "call-1",
                "project-1",
            )
            .unwrap();
        assert_eq!(*events.0.lock().unwrap(), vec![request.id]);
    }

    #[test]
    fn skill_pending_carries_provenance_and_never_creates_a_reusable_grant() {
        let (broker, grants, _audit, _events) = broker(60);
        let provenance = SkillApprovalProvenance {
            parent_agent_id: "agent-1".to_string(),
            skill_id: "review".to_string(),
            tool_id: "check".to_string(),
            effective_revision: "a".repeat(64),
            source_scope: "workspace:/project".to_string(),
            requested_capability: "tool:write_file".to_string(),
            delegated_operation: "write_file".to_string(),
            redacted_input_summary: r#"{"path":"src/lib.rs","content":"[REDACTED]"}"#.to_string(),
            immutable_witness: "sha256:witness".to_string(),
        };
        let request = broker
            .create_skill_pending(
                provenance.clone(),
                Action::file_write(),
                Resource::file_path("src/lib.rs"),
                "session-1",
                "generation-1",
                "call-1",
                "project-1",
            )
            .unwrap();

        assert_eq!(request.skill, Some(provenance));
        assert_eq!(request.risk_level, RiskLevel::L1);
        broker
            .finalize(&request.id, ApprovalDecision::Approve, Scope::Global, true)
            .unwrap();
        assert!(grants.0.lock().unwrap().is_empty());
    }

    #[test]
    fn every_skill_lifecycle_invalidation_rejects_a_late_decision() {
        for reason in [
            SkillApprovalInvalidation::Cancellation,
            SkillApprovalInvalidation::RevisionReplaced,
            SkillApprovalInvalidation::Disabled,
            SkillApprovalInvalidation::Quarantined,
            SkillApprovalInvalidation::WitnessMismatch,
        ] {
            let (broker, grants, _audit, _events) = broker(60);
            let request = broker
                .create_skill_pending(
                    SkillApprovalProvenance {
                        parent_agent_id: "agent-1".to_string(),
                        skill_id: "review".to_string(),
                        tool_id: "check".to_string(),
                        effective_revision: "a".repeat(64),
                        source_scope: "global".to_string(),
                        requested_capability: "tool:write_file".to_string(),
                        delegated_operation: "write_file".to_string(),
                        redacted_input_summary: "{}".to_string(),
                        immutable_witness: "sha256:original".to_string(),
                    },
                    Action::file_write(),
                    Resource::file_path("src/lib.rs"),
                    "session-1",
                    "generation-1",
                    "call-1",
                    "project-1",
                )
                .unwrap();
            let current = if reason == SkillApprovalInvalidation::WitnessMismatch {
                "sha256:replacement"
            } else {
                "sha256:original"
            };

            assert!(broker
                .invalidate_skill_pending(&request.id, current, reason)
                .is_some());
            assert!(broker
                .finalize(&request.id, ApprovalDecision::Approve, Scope::Global, true)
                .unwrap()
                .is_none());
            assert!(grants.0.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn mismatched_identity_cannot_invalidate_an_unrelated_skill_request() {
        let (broker, _grants, _audit, _events) = broker(60);
        let request = broker
            .create_skill_pending(
                SkillApprovalProvenance {
                    parent_agent_id: "agent-1".to_string(),
                    skill_id: "review".to_string(),
                    tool_id: "check".to_string(),
                    effective_revision: "a".repeat(64),
                    source_scope: "global".to_string(),
                    requested_capability: "tool:read_file".to_string(),
                    delegated_operation: "read_file".to_string(),
                    redacted_input_summary: "{}".to_string(),
                    immutable_witness: "sha256:exact".to_string(),
                },
                Action::file_read(),
                Resource::file_path("src/lib.rs"),
                "session-1",
                "generation-1",
                "call-1",
                "project-1",
            )
            .unwrap();

        assert!(broker
            .invalidate_skill_pending(
                &request.id,
                "sha256:similarly-named-tool",
                SkillApprovalInvalidation::Disabled,
            )
            .is_none());
        assert!(broker.get_pending(&request.id).is_some());
    }

    #[test]
    fn finalize_removes_from_pending_and_creates_a_grant_when_remembered_and_delivered() {
        let (broker, grants, audit, _events) = broker(60);
        let request = broker
            .create_pending(
                "agent-1",
                Action::file_write(),
                Resource::file_path("a.txt"),
                "session-1",
                "generation-1",
                "call-1",
                "project-1",
            )
            .unwrap();
        let resolved = broker
            .finalize(&request.id, ApprovalDecision::Approve, Scope::Session, true)
            .unwrap()
            .expect("pending approval should resolve");
        assert_eq!(resolved.effect, Effect::Allow);
        assert!(broker.get_pending(&request.id).is_none());
        assert_eq!(grants.0.lock().unwrap().len(), 1);
        assert_eq!(*audit.0.lock().unwrap(), vec![AuditDecider::Human]);
    }

    #[test]
    fn finalize_with_once_scope_does_not_create_a_grant() {
        let (broker, grants, _audit, _events) = broker(60);
        let request = broker
            .create_pending(
                "agent-1",
                Action::file_write(),
                Resource::file_path("a.txt"),
                "session-1",
                "generation-1",
                "call-1",
                "project-1",
            )
            .unwrap();
        broker
            .finalize(&request.id, ApprovalDecision::Approve, Scope::Once, true)
            .unwrap();
        assert!(grants.0.lock().unwrap().is_empty());
    }

    #[test]
    fn delegation_apply_cannot_create_a_remembered_grant() {
        let (broker, grants, _audit, _events) = broker(60);
        let request = broker
            .create_pending(
                "onepiece",
                Action::new("delegation.apply"),
                Resource::new("changeset/artifact-1"),
                "session-1",
                "generation-1",
                "call-1",
                "project-1",
            )
            .unwrap();

        broker
            .finalize(&request.id, ApprovalDecision::Approve, Scope::Global, true)
            .unwrap();

        assert!(grants.0.lock().unwrap().is_empty());
    }

    #[test]
    fn finalize_when_not_delivered_records_stale_generation_and_skips_the_grant() {
        let (broker, grants, audit, _events) = broker(60);
        let request = broker
            .create_pending(
                "agent-1",
                Action::file_write(),
                Resource::file_path("a.txt"),
                "session-1",
                "generation-1",
                "call-1",
                "project-1",
            )
            .unwrap();
        let resolved = broker
            .finalize(
                &request.id,
                ApprovalDecision::Approve,
                Scope::Session,
                false,
            )
            .unwrap()
            .expect("pending approval should still resolve, just as stale");
        assert_eq!(resolved.effect, Effect::Allow);
        assert!(grants.0.lock().unwrap().is_empty());
        assert_eq!(
            *audit.0.lock().unwrap(),
            vec![AuditDecider::StaleGeneration]
        );
    }

    #[test]
    fn finalize_on_an_unknown_request_id_returns_none() {
        let (broker, _grants, _audit, _events) = broker(60);
        let resolved = broker
            .finalize("does-not-exist", ApprovalDecision::Deny, Scope::Once, true)
            .unwrap();
        assert!(resolved.is_none());
    }

    #[test]
    fn sweep_timed_out_removes_only_expired_requests() {
        let (broker, _grants, _audit, _events) = broker(5);
        // StepClock advances by 1 each call: create_pending's internal `clock.now()` call
        // consumes tick 0.
        let old_request = broker
            .create_pending(
                "agent-1",
                Action::shell_exec(),
                Resource::workspace(),
                "session-1",
                "generation-1",
                "call-1",
                "project-1",
            )
            .unwrap();
        for _ in 0..10 {
            broker.clock.now();
        }
        let fresh_request = broker
            .create_pending(
                "agent-1",
                Action::shell_exec(),
                Resource::workspace(),
                "session-1",
                "generation-2",
                "call-2",
                "project-1",
            )
            .unwrap();
        let _ = fresh_request;

        let expired = broker.sweep_timed_out();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].id, old_request.id);
        assert_eq!(broker.list_pending().len(), 1);
    }

    #[test]
    fn sweep_timed_out_audits_each_expired_request_as_a_timeout_denial() {
        let (broker, _grants, audit, _events) = broker(0);
        broker
            .create_pending(
                "agent-1",
                Action::shell_exec(),
                Resource::workspace(),
                "session-1",
                "generation-1",
                "call-1",
                "project-1",
            )
            .unwrap();

        let expired = broker.sweep_timed_out();
        assert_eq!(expired.len(), 1);
        assert_eq!(*audit.0.lock().unwrap(), vec![AuditDecider::Timeout]);
    }
}
