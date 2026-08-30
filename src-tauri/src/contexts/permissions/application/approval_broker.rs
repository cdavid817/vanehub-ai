//! Owns the pending-approval queue as the Rust-side single source of truth (design.md D7).
//! Deliberately in-memory, not SQLite-backed: a pending approval only means anything while its
//! originating generation's process is alive, so there is nothing meaningful to recover across an
//! app restart — matching how `RuntimeAgentApiAdapter`'s own per-generation `pending_approvals`
//! already works today.

use super::error::PermissionsApplicationError;
use super::ports::{
    PendingApprovalEventPort, PermissionsClockPort, PermissionsIdPort, PrincipalRepository,
};
use crate::contexts::permissions::domain::{
    risk_level_for, Action, ApprovalRequest, PolicyTemplateName, Principal, Resource,
    SkillApprovalInvalidation, SkillApprovalProvenance,
};
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
    AlreadyClaimed { resolution_id: String },
    /// No such pending request. Distinguished from `AlreadyClaimed` because only this one means
    /// the durable ledger is the place left to look.
    NotPending,
}

/// The pending queue, and nothing that writes a decision.
///
/// It used to hold the grant and audit repositories too, because finalization lived here. Both are
/// gone: a decision becomes durable in one transaction owned by `ResolveApprovalUseCase`, and a
/// broker that could still write a grant would be a second way to create authority.
#[derive(Clone)]
pub(crate) struct ApprovalBroker {
    principals: Arc<dyn PrincipalRepository>,
    clock: Arc<dyn PermissionsClockPort>,
    ids: Arc<dyn PermissionsIdPort>,
    events: Arc<dyn PendingApprovalEventPort>,
    pending: Arc<Mutex<HashMap<String, PendingPhase>>>,
    timeout_seconds: i64,
}

impl ApprovalBroker {
    pub(crate) fn new(
        principals: Arc<dyn PrincipalRepository>,
        clock: Arc<dyn PermissionsClockPort>,
        ids: Arc<dyn PermissionsIdPort>,
        events: Arc<dyn PendingApprovalEventPort>,
        timeout_seconds: i64,
    ) -> Self {
        Self {
            principals,
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

    /// Removes a pending entry whatever phase it is in.
    ///
    /// For the emergency denial path only, which releases the waiter without a durable record.
    /// Reverting the claim there would leave the request on offer after its waiter was already
    /// denied, letting a human "approve" something that has been refused.
    pub(crate) fn discard_pending(&self, request_id: &str) -> bool {
        self.lock_pending().remove(request_id).is_some()
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

    /// One pending approval, whatever phase it is in.
    ///
    /// No production caller today: the frontend reads `list_pending`, and the resolver claims by id
    /// rather than looking first. Kept because it is what the tests assert queue state with, and a
    /// single-request read is the natural companion to the list.
    #[cfg_attr(not(test), expect(dead_code))]
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

    /// Which pending approvals have waited past the timeout window.
    ///
    /// Reports rather than resolves, and that is the change. It used to remove each expired request
    /// and write its own denial audit, which made the sweep a second decision engine racing the
    /// human one. Now the caller feeds these ids back through the same claim/commit/deliver use
    /// case a human `Deny` uses, so a timeout that arrives while somebody is clicking loses the
    /// claim instead of writing a competing resolution.
    ///
    /// Claimed entries are skipped for the same reason: a request somebody is midway through
    /// resolving is not waiting for anyone.
    pub(crate) fn expired_pending_ids(&self) -> Vec<String> {
        let now: i64 = self.clock.now().parse().unwrap_or(0);
        self.lock_pending()
            .values()
            .filter_map(|phase| match phase {
                PendingPhase::Pending(request) => Some(request),
                PendingPhase::Resolving { .. } | PendingPhase::Committed { .. } => None,
            })
            .filter(|request| {
                let created_at: i64 = request.created_at.parse().unwrap_or(now);
                now.saturating_sub(created_at) >= self.timeout_seconds
            })
            .map(|request| request.id.clone())
            .collect()
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

    fn broker(timeout_seconds: i64) -> (ApprovalBroker, Arc<FakeEvents>) {
        let events = Arc::new(FakeEvents::default());
        let broker = ApprovalBroker::new(
            Arc::new(FakePrincipals::default()),
            Arc::new(StepClock(StdMutex::new(0))),
            Arc::new(FakeIds(StdMutex::new(0))),
            events.clone(),
            timeout_seconds,
        );
        (broker, events)
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
        let (broker, _events) = broker(60);
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
        let (broker, events) = broker(60);
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
        let (broker, _events) = broker(60);
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
        // The forced-Once rule that keeps this from becoming a reusable grant lives in
        // `ResolveApprovalUseCase` and is asserted there; the broker's job is to carry the
        // provenance that makes the request recognisable as a delegated one.
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
            let (broker, _events) = broker(60);
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
            // An invalidated request is gone from the queue, so a decision arriving afterwards has
            // nothing to claim — which is what stops a late click from resolving a Skill whose
            // revision was replaced underneath it.
            assert!(matches!(
                broker.claim(&request.id, "resolution-1"),
                ApprovalClaim::NotPending
            ));
        }
    }

    #[test]
    fn mismatched_identity_cannot_invalidate_an_unrelated_skill_request() {
        let (broker, _events) = broker(60);
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
    fn only_requests_past_the_timeout_window_are_reported_as_expired() {
        let (broker, _events) = broker(5);
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

        let expired = broker.expired_pending_ids();

        assert_eq!(expired, vec![old_request.id]);
        // Reported, not removed. The resolver claims each id through the same path a human
        // decision uses, so the entry has to still be there for it to claim.
        assert_eq!(broker.list_pending().len(), 2);
    }

    #[test]
    fn only_one_caller_can_claim_a_pending_request() {
        let (broker, _events) = broker(60);
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
        let (broker, _events) = broker(60);
        // The distinction matters to the caller: only this one means the durable ledger is the
        // place left to look for an answer.
        assert!(matches!(
            broker.claim("does-not-exist", "resolution-1"),
            ApprovalClaim::NotPending
        ));
    }

    #[test]
    fn only_the_claimant_can_revert_or_commit_its_claim() {
        let (broker, _events) = broker(60);
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
        let (broker, _events) = broker(60);
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
        let (broker, _events) = broker(60);
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
        let (broker, _events) = broker(60);
        let request = pending(&broker);
        broker.claim(&request.id, "resolution-1");

        // The row must not vanish and reappear while somebody is resolving it — the frontend's
        // pull is what reconciles an ambiguous response, and it can only reconcile what it sees.
        assert_eq!(broker.list_pending().len(), 1);
        assert!(broker.get_pending(&request.id).is_some());
    }

    #[test]
    fn a_timeout_sweep_leaves_a_request_somebody_is_already_resolving() {
        let (broker, _events) = broker(0);
        let request = pending(&broker);
        broker.claim(&request.id, "resolution-1");

        // Reporting a claimed request as expired would send the sweep to race a human decision
        // for the same claim, which is exactly what the single-winner phase exists to prevent.
        assert!(broker.expired_pending_ids().is_empty());
        assert!(broker.get_pending(&request.id).is_some());
    }

    #[test]
    fn an_expired_request_stays_in_the_queue_for_the_resolver_to_claim() {
        let (broker, _events) = broker(0);
        let request = pending(&broker);

        let expired = broker.expired_pending_ids();

        assert_eq!(expired, vec![request.id.clone()]);
        // The denial's audit row belongs to the resolution transaction that commits it, not to the
        // sweep that noticed the expiry — so the sweep leaves the entry alone and the resolver
        // claims it exactly as a human decision would.
        assert!(matches!(
            broker.claim(&request.id, "resolution-1"),
            ApprovalClaim::Claimed(_)
        ));
    }
}
