//! Owns the pending-approval queue as the Rust-side single source of truth (design.md D7).
//! Deliberately in-memory, not SQLite-backed: a pending approval only means anything while its
//! originating generation's process is alive, so there is nothing meaningful to recover across an
//! app restart — matching how `RuntimeAgentApiAdapter`'s own per-generation `pending_approvals`
//! already works today.

use super::error::PermissionsApplicationError;
use super::ports::{
    AuditDecider, AuditRecord, AuditRepository, GrantRepository, PendingApprovalEventPort,
    PendingGrantIntent, PermissionsClockPort, PermissionsIdPort, PrincipalRepository,
};
use crate::contexts::permissions::domain::{
    risk_level_for, Action, ApprovalDecision, ApprovalRequest, CanonicalGrantKey, Effect,
    PersistedEffect, PolicyTemplateName, Principal, RememberedScope, Resource, Scope,
    SkillApprovalInvalidation, SkillApprovalProvenance,
};
#[cfg(test)]
use crate::contexts::permissions::domain::{Grant, GrantActivationState};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

/// Where one pending approval is in the act of being resolved.
///
/// The queue used to hold bare requests and finalization began by removing one. That made taking
/// the request out of the map the first irreversible step: a storage failure afterwards left the
/// decision nowhere — the caller held an error it could not act on, and a retry found nothing to
/// retry. Naming the intermediate state is what makes a pre-commit failure recoverable.
enum PendingPhase {
    Pending(ApprovalRequest),
    /// Claimed by exactly one caller, which holds `resolution_id`. Nothing durable has been
    /// written yet, so this phase can still be reverted — but only by the claimant.
    Resolving {
        request: ApprovalRequest,
        resolution_id: String,
    },
    /// The decision is durable. Never reverted: from here the request has an answer, and a second
    /// caller must be told that answer rather than offered a fresh decision.
    Committed {
        request: ApprovalRequest,
        resolution_id: String,
    },
}

impl PendingPhase {
    fn request(&self) -> &ApprovalRequest {
        match self {
            Self::Pending(request)
            | Self::Resolving { request, .. }
            | Self::Committed { request, .. } => request,
        }
    }
}

/// What a claim attempt found.
pub(crate) enum ApprovalClaim {
    /// This caller now owns the resolution and carries the id it must commit under.
    Claimed(Box<ApprovalRequest>),
    /// Somebody else got there first. Carries their resolution id so the caller can report the
    /// existing outcome instead of producing a competing one.
    AlreadyClaimed {
        /// Read by `ResolveApprovalUseCase` once it exists (task 5.2/5.3), which answers a losing
        /// caller with the winner's durable state. `finalize` currently only needs to know that it
        /// lost, so nothing in production reads the id yet.
        #[cfg_attr(not(test), expect(dead_code))]
        resolution_id: String,
    },
    /// No such pending request. Distinguished from `AlreadyClaimed` because only this one means
    /// the durable ledger is the place left to look.
    NotPending,
}

#[derive(Clone)]
pub(crate) struct ApprovalBroker {
    principals: Arc<dyn PrincipalRepository>,
    grants: Arc<dyn GrantRepository>,
    audit: Arc<dyn AuditRepository>,
    clock: Arc<dyn PermissionsClockPort>,
    ids: Arc<dyn PermissionsIdPort>,
    events: Arc<dyn PendingApprovalEventPort>,
    pending: Arc<Mutex<HashMap<String, PendingPhase>>>,
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
    fn lock_pending(&self) -> MutexGuard<'_, HashMap<String, PendingPhase>> {
        self.pending.lock().unwrap_or_else(|poisoned| {
            debug_assert!(false, "pending approvals mutex poisoned");
            poisoned.into_inner()
        })
    }

    /// Takes single-winner ownership of one pending request.
    ///
    /// Atomic under the pending mutex, so two callers submitting opposite decisions at the same
    /// moment cannot both proceed: one gets the request, the other gets the winner's resolution id
    /// and reports that result rather than writing a second one.
    pub(crate) fn claim(&self, request_id: &str, resolution_id: &str) -> ApprovalClaim {
        let mut pending = self.lock_pending();
        match pending.remove(request_id) {
            Some(PendingPhase::Pending(request)) => {
                pending.insert(
                    request_id.to_string(),
                    PendingPhase::Resolving {
                        request: request.clone(),
                        resolution_id: resolution_id.to_string(),
                    },
                );
                ApprovalClaim::Claimed(Box::new(request))
            }
            Some(claimed) => {
                let resolution_id = match &claimed {
                    PendingPhase::Resolving { resolution_id, .. }
                    | PendingPhase::Committed { resolution_id, .. } => resolution_id.clone(),
                    PendingPhase::Pending(_) => unreachable!("matched above"),
                };
                pending.insert(request_id.to_string(), claimed);
                ApprovalClaim::AlreadyClaimed { resolution_id }
            }
            None => ApprovalClaim::NotPending,
        }
    }

    /// Returns a claim to `Pending` after a failure that wrote nothing.
    ///
    /// Compare-and-revert: only the holder of `resolution_id` may release it. Without that check a
    /// late failure from an abandoned attempt could unlock a request another caller is midway
    /// through committing.
    pub(crate) fn revert_claim(&self, request_id: &str, resolution_id: &str) -> bool {
        let mut pending = self.lock_pending();
        match pending.get(request_id) {
            Some(PendingPhase::Resolving {
                request,
                resolution_id: held,
            }) if held == resolution_id => {
                let request = request.clone();
                pending.insert(request_id.to_string(), PendingPhase::Pending(request));
                true
            }
            _ => false,
        }
    }

    /// Marks a claim durable. After this the entry is never returned to `Pending`: the decision
    /// exists, and offering it again would invite a second, conflicting one.
    pub(crate) fn mark_committed(&self, request_id: &str, resolution_id: &str) -> bool {
        let mut pending = self.lock_pending();
        match pending.get(request_id) {
            Some(PendingPhase::Resolving {
                request,
                resolution_id: held,
            }) if held == resolution_id => {
                let request = request.clone();
                pending.insert(
                    request_id.to_string(),
                    PendingPhase::Committed {
                        request,
                        resolution_id: resolution_id.to_string(),
                    },
                );
                true
            }
            _ => false,
        }
    }

    /// Drops a committed entry once its delivery has been resolved one way or the other.
    pub(crate) fn release_committed(&self, request_id: &str, resolution_id: &str) -> bool {
        let mut pending = self.lock_pending();
        match pending.get(request_id) {
            Some(PendingPhase::Committed {
                resolution_id: held,
                ..
            }) if held == resolution_id => pending.remove(request_id).is_some(),
            _ => false,
        }
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
            .insert(request.id.clone(), PendingPhase::Pending(request.clone()));
        // Best-effort (see `PendingApprovalEventPort`'s own doc comment): a publish failure must
        // not fail approval creation itself, since the frontend's pull-on-mount already covers
        // a missed event.
        let _ = self.events.publish(&request);
        Ok(request)
    }

    /// The full pending list — `permissions-approval`'s "Pending approval state is Rust-side
    /// authoritative" and its pull-reconciliation-on-mount requirement both read this.
    ///
    /// Includes claimed and committed entries. A request being resolved is still a request the
    /// frontend has to be able to see: dropping it from the list the moment somebody clicked would
    /// make the row vanish and reappear, and the pull is what reconciles an ambiguous response.
    pub(crate) fn list_pending(&self) -> Vec<ApprovalRequest> {
        self.lock_pending()
            .values()
            .map(|phase| phase.request().clone())
            .collect()
    }

    pub(crate) fn get_pending(&self, request_id: &str) -> Option<ApprovalRequest> {
        self.lock_pending()
            .get(request_id)
            .map(|phase| phase.request().clone())
    }

    /// Whether this request already has a claimant, and under which resolution id.
    ///
    /// What the frontend's `resolving` state will be derived from (`permissions-approval`'s
    /// "Approval is being committed"): while a claim exists, Approve and Deny are disabled so a
    /// second competing decision cannot be submitted. The DTO that carries it to the frontend is
    /// task 8.1's, so no production caller reads this yet.
    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn claimed_resolution_id(&self, request_id: &str) -> Option<String> {
        match self.lock_pending().get(request_id) {
            Some(PendingPhase::Resolving { resolution_id, .. })
            | Some(PendingPhase::Committed { resolution_id, .. }) => Some(resolution_id.clone()),
            _ => None,
        }
    }

    pub(crate) fn invalidate_skill_pending(
        &self,
        request_id: &str,
        current_witness: &str,
        reason: SkillApprovalInvalidation,
    ) -> Option<ApprovalRequest> {
        let mut pending = self.lock_pending();
        let request = pending.get(request_id)?.request();
        let skill = request.skill.as_ref()?;
        let witness_matches = skill.immutable_witness == current_witness;
        let invalid = match reason {
            SkillApprovalInvalidation::WitnessMismatch => !witness_matches,
            SkillApprovalInvalidation::Cancellation
            | SkillApprovalInvalidation::RevisionReplaced
            | SkillApprovalInvalidation::Disabled
            | SkillApprovalInvalidation::Quarantined => witness_matches,
        };
        invalid
            .then(|| pending.remove(request_id))
            .flatten()
            .map(|phase| phase.request().clone())
    }

    /// Finalizes a pending approval. `delivered` distinguishes two outcomes the caller (the
    /// `permissions` command handler, per design.md D8's refinement) determines by first calling
    /// `AgentRuntimeApi::resolve_tool_approval`: `true` means a live generation was actually
    /// unblocked (this was a genuine human decision — `AuditDecider::Human`, and a grant is
    /// created if `scope` is remembered); `false` means the generation had already ended (nothing
    /// to unblock — `AuditDecider::StaleGeneration`, no grant, matching design.md D6). Returns
    /// `None` if `request_id` names no pending approval (already resolved, or never existed).
    ///
    /// Claims before it writes, and reverts the claim if nothing became durable. The previous
    /// implementation removed the request first, which made a storage failure unrecoverable: the
    /// user's decision existed only in the map that had just been emptied.
    pub(crate) fn finalize(
        &self,
        request_id: &str,
        decision: ApprovalDecision,
        scope: Scope,
        delivered: bool,
    ) -> Result<Option<ResolvedApproval>, PermissionsApplicationError> {
        let resolution_id = self.ids.next_id("resolution");
        let request = match self.claim(request_id, &resolution_id) {
            ApprovalClaim::Claimed(request) => *request,
            // Another caller owns this decision. Reporting its outcome rather than writing a
            // second one is what makes a double click one resolution instead of two.
            ApprovalClaim::AlreadyClaimed { .. } | ApprovalClaim::NotPending => return Ok(None),
        };

        match self.write_resolution(&request, &resolution_id, decision, scope, delivered) {
            Ok(effect) => {
                self.mark_committed(request_id, &resolution_id);
                self.release_committed(request_id, &resolution_id);
                Ok(Some(ResolvedApproval { request, effect }))
            }
            Err(error) => {
                // Nothing durable was written, so the decision is still the user's to make and the
                // request has to go back on offer.
                self.revert_claim(request_id, &resolution_id);
                Err(error)
            }
        }
    }

    /// The durable half of finalization: remembered-grant intent first, then the decision audit.
    ///
    /// Ordered so a grant failure cannot leave an audit row claiming a decision that was never
    /// recorded. Group 4 replaces both writes with one transaction; until then the ordering is
    /// what bounds the damage.
    fn write_resolution(
        &self,
        request: &ApprovalRequest,
        resolution_id: &str,
        decision: ApprovalDecision,
        scope: Scope,
        delivered: bool,
    ) -> Result<Effect, PermissionsApplicationError> {
        let effect = decision.as_effect();
        let decider = if delivered {
            AuditDecider::Human
        } else {
            AuditDecider::StaleGeneration
        };
        // Skills and delegation are authorised for one use only, whatever the caller asked for.
        // Enforced here rather than trusted from the request so a new caller cannot widen it.
        let scope = if request.action.as_str() == "delegation.apply" || request.skill.is_some() {
            Scope::Once
        } else {
            scope
        };
        if delivered {
            if let Some(intent) = self.grant_intent(request, resolution_id, effect, scope)? {
                self.grants.upsert_pending_grant_intent(&intent)?;
                self.grants
                    .activate_grant_for_resolution(resolution_id, &self.clock.now())?;
            }
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
        Ok(effect)
    }

    /// What this decision should remember, if anything.
    ///
    /// `None` covers every decision that is not rememberable — `Once`, and an effect that is not
    /// Allow or Deny. Both are refusals by the domain rather than checks repeated here, so a new
    /// scope or effect cannot quietly become persistable by being forgotten at this call site.
    fn grant_intent(
        &self,
        request: &ApprovalRequest,
        resolution_id: &str,
        effect: Effect,
        scope: Scope,
    ) -> Result<Option<PendingGrantIntent>, PermissionsApplicationError> {
        // A request always carries both a session and a project. Which of them owns the grant is
        // decided by the scope alone, so the other is cleared rather than passed through — a
        // binding that named both would be rejected, and rightly so.
        let binding = match scope {
            Scope::Once => return Ok(None),
            Scope::Session => {
                RememberedScope::parse(scope, Some(request.session_id.as_str()), None)?
            }
            Scope::Project => {
                RememberedScope::parse(scope, None, Some(request.project_key.as_str()))?
            }
            Scope::Global => RememberedScope::parse(scope, None, None)?,
        };
        let Ok(effect) = PersistedEffect::parse(effect) else {
            return Ok(None);
        };
        let key = CanonicalGrantKey::new(
            request.principal_id.clone(),
            request.action.clone(),
            request.resource.clone(),
            binding,
        )?;
        Ok(Some(PendingGrantIntent {
            id: self.ids.next_id("grant"),
            key,
            effect,
            resolution_id: resolution_id.to_string(),
            now: self.clock.now(),
        }))
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
                // Only unclaimed entries. A request somebody is midway through resolving is not
                // waiting for anyone — sweeping it would race a human decision and produce a second
                // one, which is exactly what the single-winner claim exists to prevent.
                .filter_map(|phase| match phase {
                    PendingPhase::Pending(request) => Some(request),
                    PendingPhase::Resolving { .. } | PendingPhase::Committed { .. } => None,
                })
                .filter(|request| {
                    let created_at: i64 = request.created_at.parse().unwrap_or(now);
                    now.saturating_sub(created_at) >= self.timeout_seconds
                })
                .map(|request| request.id.clone())
                .collect();
            expired_ids
                .into_iter()
                .filter_map(|id| pending.remove(&id))
                .map(|phase| phase.request().clone())
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
        // Atomic for the same reason `EvaluationService` uses it: two generations meeting a new
        // agent together would otherwise have one lose the unique-`agent_id` insert.
        self.principals.get_or_create(
            agent_id,
            &self.ids.next_id("principal"),
            PolicyTemplateName::Standard,
        )
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
        fn get_or_create(
            &self,
            agent_id: &str,
            id_hint: &str,
            default_template: PolicyTemplateName,
        ) -> Result<Principal, PermissionsApplicationError> {
            let mut principals = self.0.lock().unwrap();
            if let Some(principal) = principals.get(agent_id) {
                return Ok(principal.clone());
            }
            let principal = Principal::new(
                id_hint.to_string(),
                agent_id.to_string(),
                default_template,
                None,
                None,
            )?;
            principals.insert(agent_id.to_string(), principal.clone());
            Ok(principal)
        }
        fn update_template(
            &self,
            _principal_id: &str,
            _template: PolicyTemplateName,
        ) -> Result<(), PermissionsApplicationError> {
            Ok(())
        }
    }

    /// Records every intent it is handed and which resolutions were activated, so a test can tell
    /// "an intent was written" apart from "a grant became visible to evaluation" — the whole point
    /// of the two-phase activation.
    #[derive(Default)]
    struct FakeGrants {
        intents: StdMutex<Vec<PendingGrantIntent>>,
        activated: StdMutex<Vec<String>>,
    }
    impl GrantRepository for FakeGrants {
        fn find_effective_grant(
            &self,
            _query: &super::super::ports::GrantQuery<'_>,
        ) -> Result<Option<Grant>, PermissionsApplicationError> {
            Ok(None)
        }
        fn upsert_pending_grant_intent(
            &self,
            intent: &PendingGrantIntent,
        ) -> Result<Grant, PermissionsApplicationError> {
            let grant = Grant {
                id: intent.id.clone(),
                key: intent.key.clone(),
                effect: intent.effect,
                revision: 1,
                activation_state: GrantActivationState::PendingDelivery,
                resolution_id: Some(intent.resolution_id.clone()),
                created_at: intent.now.clone(),
                updated_at: intent.now.clone(),
            };
            self.intents.lock().unwrap().push(PendingGrantIntent {
                id: intent.id.clone(),
                key: intent.key.clone(),
                effect: intent.effect,
                resolution_id: intent.resolution_id.clone(),
                now: intent.now.clone(),
            });
            Ok(grant)
        }
        fn activate_grant_for_resolution(
            &self,
            resolution_id: &str,
            _now: &str,
        ) -> Result<(), PermissionsApplicationError> {
            self.activated
                .lock()
                .unwrap()
                .push(resolution_id.to_string());
            Ok(())
        }
    }

    /// A grant store that is down for the first `failures` writes and healthy afterwards.
    ///
    /// Stands in for the storage outage the atomic-resolution requirement is written against. The
    /// recovery matters as much as the failure: "the decision did not become durable" is only half
    /// the requirement, and the other half is that the user's approval survives to be retried.
    #[derive(Default)]
    struct FlakyGrants {
        remaining_failures: StdMutex<usize>,
        intents: StdMutex<Vec<String>>,
    }
    impl FlakyGrants {
        fn failing(times: usize) -> Self {
            Self {
                remaining_failures: StdMutex::new(times),
                intents: StdMutex::new(Vec::new()),
            }
        }
    }
    impl GrantRepository for FlakyGrants {
        fn find_effective_grant(
            &self,
            _query: &super::super::ports::GrantQuery<'_>,
        ) -> Result<Option<Grant>, PermissionsApplicationError> {
            Ok(None)
        }
        fn upsert_pending_grant_intent(
            &self,
            intent: &PendingGrantIntent,
        ) -> Result<Grant, PermissionsApplicationError> {
            let mut remaining = self.remaining_failures.lock().unwrap();
            if *remaining > 0 {
                *remaining -= 1;
                return Err(PermissionsApplicationError::infrastructure(
                    "sqlite",
                    "grant store unavailable".to_string(),
                ));
            }
            self.intents.lock().unwrap().push(intent.id.clone());
            Ok(Grant {
                id: intent.id.clone(),
                key: intent.key.clone(),
                effect: intent.effect,
                revision: 1,
                activation_state: GrantActivationState::PendingDelivery,
                resolution_id: Some(intent.resolution_id.clone()),
                created_at: intent.now.clone(),
                updated_at: intent.now.clone(),
            })
        }
        fn activate_grant_for_resolution(
            &self,
            _resolution_id: &str,
            _now: &str,
        ) -> Result<(), PermissionsApplicationError> {
            Ok(())
        }
    }

    /// A grant store that is down. Stands in for the storage outage the atomic-resolution
    /// requirement is written against — the point is not which statement failed but that the
    /// decision did not become durable.
    struct UnavailableGrants;
    impl GrantRepository for UnavailableGrants {
        fn find_effective_grant(
            &self,
            _query: &super::super::ports::GrantQuery<'_>,
        ) -> Result<Option<Grant>, PermissionsApplicationError> {
            Ok(None)
        }
        fn upsert_pending_grant_intent(
            &self,
            _intent: &PendingGrantIntent,
        ) -> Result<Grant, PermissionsApplicationError> {
            Err(PermissionsApplicationError::infrastructure(
                "sqlite",
                "grant store unavailable".to_string(),
            ))
        }
        fn activate_grant_for_resolution(
            &self,
            _resolution_id: &str,
            _now: &str,
        ) -> Result<(), PermissionsApplicationError> {
            Err(PermissionsApplicationError::infrastructure(
                "sqlite",
                "grant store unavailable".to_string(),
            ))
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

    /// One ordinary pending approval, so a test about claiming states only what it is about.
    fn pending(broker: &ApprovalBroker) -> ApprovalRequest {
        broker
            .create_pending(
                "agent-1",
                Action::file_write(),
                Resource::file_path("a.txt"),
                "session-1",
                "generation-1",
                "call-1",
                "project-1",
            )
            .expect("create pending")
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
        assert!(grants.intents.lock().unwrap().is_empty());
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
            assert!(grants.intents.lock().unwrap().is_empty());
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
        assert_eq!(grants.intents.lock().unwrap().len(), 1);
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
        assert!(grants.intents.lock().unwrap().is_empty());
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

        assert!(grants.intents.lock().unwrap().is_empty());
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
        assert!(grants.intents.lock().unwrap().is_empty());
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

    /// Characterization for `permissions-core`'s "Grant write fails inside resolution transaction"
    /// and the invariant that removing the pending request is never the first irreversible step.
    ///
    /// Finalization takes the request out of the pending map before it writes anything. When the
    /// grant write then fails, the decision is gone from the only place that still knew about it:
    /// the caller holds an error it cannot act on, a retry finds nothing, and the audit row that
    /// would have recorded the attempt was never reached either. The approval is neither applied
    /// nor recoverable.
    #[test]
    fn a_failed_grant_write_leaves_the_approval_retryable_and_unaudited() {
        let audit = Arc::new(FakeAudit::default());
        // Down for exactly one write. The recovery is half the requirement: a decision that cannot
        // be retried is as lost as one that was never made.
        let grants = Arc::new(FlakyGrants::failing(1));
        let broker = ApprovalBroker::new(
            Arc::new(FakePrincipals::default()),
            grants.clone(),
            audit.clone(),
            Arc::new(StepClock(StdMutex::new(0))),
            Arc::new(FakeIds(StdMutex::new(0))),
            Arc::new(FakeEvents::default()),
            60,
        );
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

        let failed = broker.finalize(&request.id, ApprovalDecision::Approve, Scope::Session, true);
        assert!(failed.is_err(), "the grant store was down");
        assert!(grants.intents.lock().unwrap().is_empty());

        // Nothing durable was written, so the decision has to still be resolvable. A pending entry
        // consumed by a failed attempt is an approval the user made that the system can neither
        // honour nor be asked about again.
        assert!(
            broker.get_pending(&request.id).is_some(),
            "the failed resolution consumed the pending request, so it cannot be retried"
        );
        assert!(
            audit.0.lock().unwrap().is_empty(),
            "a failed resolution must not leave a partial audit trail"
        );

        // And the retry, once storage is the only thing that changed, must be able to complete.
        let retried = broker.finalize(&request.id, ApprovalDecision::Approve, Scope::Session, true);
        assert!(
            retried.is_ok_and(|resolved| resolved.is_some()),
            "a resolution that failed before commit must be retryable"
        );
        assert_eq!(grants.intents.lock().unwrap().len(), 1);
        assert_eq!(*audit.0.lock().unwrap(), vec![AuditDecider::Human]);
        assert!(broker.get_pending(&request.id).is_none());
    }

    /// `permissions-approval`'s "Two frontends resolve the same request concurrently".
    ///
    /// Not raced for: the first claim is taken and held, which is the state a second caller
    /// actually meets when two clicks land close together. Exactly one of them owns the decision.
    #[test]
    fn only_one_caller_can_claim_a_pending_request() {
        let (broker, _grants, _audit, _events) = broker(60);
        let request = pending(&broker);

        let first = broker.claim(&request.id, "resolution-1");
        let second = broker.claim(&request.id, "resolution-2");

        assert!(matches!(first, ApprovalClaim::Claimed(_)));
        match second {
            ApprovalClaim::AlreadyClaimed { resolution_id } => {
                // The loser is told the winner's id rather than "not found": it has to report the
                // existing outcome, not offer the user a second decision.
                assert_eq!(resolution_id, "resolution-1");
            }
            _ => panic!("a second caller claimed a request that was already owned"),
        }
        assert_eq!(
            broker.claimed_resolution_id(&request.id).as_deref(),
            Some("resolution-1")
        );
    }

    #[test]
    fn claiming_a_request_that_was_never_pending_is_distinguishable_from_losing_a_race() {
        let (broker, _grants, _audit, _events) = broker(60);
        // The distinction matters to the caller: only this one means the durable ledger is the
        // place left to look for an answer.
        assert!(matches!(
            broker.claim("does-not-exist", "resolution-1"),
            ApprovalClaim::NotPending
        ));
    }

    #[test]
    fn only_the_claimant_can_revert_or_commit_its_claim() {
        let (broker, _grants, _audit, _events) = broker(60);
        let request = pending(&broker);
        broker.claim(&request.id, "resolution-1");

        // A late failure from an abandoned attempt must not unlock a request somebody else is
        // midway through committing.
        assert!(!broker.revert_claim(&request.id, "resolution-2"));
        assert!(!broker.mark_committed(&request.id, "resolution-2"));
        assert_eq!(
            broker.claimed_resolution_id(&request.id).as_deref(),
            Some("resolution-1")
        );

        assert!(broker.revert_claim(&request.id, "resolution-1"));
        assert_eq!(broker.claimed_resolution_id(&request.id), None);
    }

    #[test]
    fn a_reverted_claim_puts_the_request_back_on_offer() {
        let (broker, _grants, _audit, _events) = broker(60);
        let request = pending(&broker);
        broker.claim(&request.id, "resolution-1");
        broker.revert_claim(&request.id, "resolution-1");

        assert!(matches!(
            broker.claim(&request.id, "resolution-2"),
            ApprovalClaim::Claimed(_)
        ));
    }

    #[test]
    fn a_committed_claim_is_never_returned_to_pending() {
        let (broker, _grants, _audit, _events) = broker(60);
        let request = pending(&broker);
        broker.claim(&request.id, "resolution-1");
        assert!(broker.mark_committed(&request.id, "resolution-1"));

        // The decision is durable from here. Reverting would offer the user a second decision for
        // a request that already has an answer.
        assert!(!broker.revert_claim(&request.id, "resolution-1"));
        assert!(matches!(
            broker.claim(&request.id, "resolution-2"),
            ApprovalClaim::AlreadyClaimed { .. }
        ));
        assert!(broker.release_committed(&request.id, "resolution-1"));
        assert!(broker.get_pending(&request.id).is_none());
    }

    #[test]
    fn a_claimed_request_is_still_visible_to_the_pending_list() {
        let (broker, _grants, _audit, _events) = broker(60);
        let request = pending(&broker);
        broker.claim(&request.id, "resolution-1");

        // The row must not vanish and reappear while somebody is resolving it — the frontend's
        // pull is what reconciles an ambiguous response, and it can only reconcile what it sees.
        assert_eq!(broker.list_pending().len(), 1);
        assert!(broker.get_pending(&request.id).is_some());
    }

    #[test]
    fn a_timeout_sweep_leaves_a_request_somebody_is_already_resolving() {
        let (broker, _grants, audit, _events) = broker(0);
        let request = pending(&broker);
        broker.claim(&request.id, "resolution-1");

        let expired = broker.sweep_timed_out();

        // Sweeping a claimed request would race the human decision and produce a second one.
        assert!(expired.is_empty());
        assert!(audit.0.lock().unwrap().is_empty());
        assert!(broker.get_pending(&request.id).is_some());
    }

    #[test]
    fn a_delivered_remembered_decision_writes_an_intent_and_activates_it() {
        let (broker, grants, _audit, _events) = broker(60);
        let request = pending(&broker);

        broker
            .finalize(&request.id, ApprovalDecision::Approve, Scope::Session, true)
            .unwrap();

        let intents = grants.intents.lock().unwrap();
        assert_eq!(intents.len(), 1);
        assert_eq!(intents[0].effect, PersistedEffect::Allow);
        assert_eq!(
            intents[0].key.scope,
            RememberedScope::Session("session-1".to_string()),
            "the session scope did not bind the grant to the request's session"
        );
        // Activation is addressed by resolution id, which is what the delivery acknowledgement
        // will carry.
        assert_eq!(
            *grants.activated.lock().unwrap(),
            vec![intents[0].resolution_id.clone()]
        );
    }

    #[test]
    fn a_project_scoped_decision_binds_to_the_project_and_not_the_session() {
        let (broker, grants, _audit, _events) = broker(60);
        let request = pending(&broker);

        broker
            .finalize(&request.id, ApprovalDecision::Deny, Scope::Project, true)
            .unwrap();

        let intents = grants.intents.lock().unwrap();
        assert_eq!(
            intents[0].key.scope,
            RememberedScope::Project("project-1".to_string())
        );
        assert_eq!(intents[0].effect, PersistedEffect::Deny);
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
