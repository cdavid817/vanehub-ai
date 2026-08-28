//! Ordering proofs for `ResolveApprovalUseCase`.
//!
//! Almost every test here is about *when* something happened rather than whether it succeeded, so
//! the doubles record an ordered event log rather than a set of flags. The single most important
//! assertion in the file is that no `deliver` event ever appears before its `commit` event: that is
//! the property the whole change exists to establish, and it is not visible from any single
//! repository's state afterwards.

use super::*;
use crate::contexts::permissions::application::ports::{
    PendingApprovalEventPort, PrincipalRepository,
};
use crate::contexts::permissions::domain::{
    Action, PolicyTemplateName, Principal, Resource, SkillApprovalProvenance,
};
use std::collections::HashMap;
use std::sync::Mutex;

/// What happened, in order. The ordering is the assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    Reserved,
    Committed(String),
    Delivered(String),
    Acknowledged(String),
    DeliveryFailed(String),
}

#[derive(Default)]
struct Journal(Mutex<Vec<Event>>);

impl Journal {
    fn push(&self, event: Event) {
        self.0.lock().unwrap().push(event);
    }

    fn events(&self) -> Vec<Event> {
        self.0.lock().unwrap().clone()
    }
}

#[derive(Default)]
struct FakePrincipals(Mutex<HashMap<String, Principal>>);
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

// No grant or audit double here, deliberately: the broker no longer holds either, and this use
// case writes both through `commit_resolution`. A test that could satisfy a grant write outside
// that transaction would be asserting against a path production does not have.

#[derive(Default)]
struct SilentEvents;
impl PendingApprovalEventPort for SilentEvents {
    fn publish(&self, _request: &ApprovalRequest) -> Result<(), PermissionsApplicationError> {
        Ok(())
    }
}

struct StepClock(Mutex<i64>);
impl PermissionsClockPort for StepClock {
    fn now(&self) -> String {
        let mut value = self.0.lock().unwrap();
        *value += 1;
        value.to_string()
    }
}

/// Counts per prefix, not globally.
///
/// A single counter would make `resolution-1` depend on how many approvals, principals and audit
/// rows happened to be allocated first — so adding an unrelated `next_id` call anywhere would
/// break every assertion in this file that names an id.
#[derive(Default)]
struct SequentialIds(Mutex<HashMap<String, u64>>);
impl PermissionsIdPort for SequentialIds {
    fn next_id(&self, prefix: &str) -> String {
        let mut counters = self.0.lock().unwrap();
        let counter = counters.entry(prefix.to_string()).or_default();
        *counter += 1;
        format!("{prefix}-{counter}")
    }
}

/// An in-memory stand-in for the durable ledger, with the same guarded transitions.
struct FakeResolutions {
    journal: Arc<Journal>,
    rows: Mutex<Vec<(ApprovalResolution, bool)>>,
    fail_commits: Mutex<usize>,
    grant_active: Mutex<bool>,
    grant_intents: Mutex<Vec<PendingGrantIntent>>,
}

impl FakeResolutions {
    fn new(journal: Arc<Journal>) -> Self {
        Self {
            journal,
            rows: Mutex::new(Vec::new()),
            fail_commits: Mutex::new(0),
            grant_active: Mutex::new(false),
            grant_intents: Mutex::new(Vec::new()),
        }
    }

    fn failing_commits(journal: Arc<Journal>, times: usize) -> Self {
        let repository = Self::new(journal);
        *repository.fail_commits.lock().unwrap() = times;
        repository
    }

    fn state_of(&self, request_id: &str) -> Option<ApprovalResolutionState> {
        self.rows
            .lock()
            .unwrap()
            .iter()
            .find(|(row, _)| row.request_id == request_id)
            .map(|(row, _)| row.state)
    }

    fn grant_is_active(&self) -> bool {
        *self.grant_active.lock().unwrap()
    }
}

impl ApprovalResolutionRepository for FakeResolutions {
    fn commit_resolution(
        &self,
        commit: &ResolutionCommit,
    ) -> Result<ApprovalResolution, PermissionsApplicationError> {
        let mut remaining = self.fail_commits.lock().unwrap();
        if *remaining > 0 {
            *remaining -= 1;
            return Err(PermissionsApplicationError::infrastructure(
                "sqlite",
                "resolution store unavailable".to_string(),
            ));
        }
        drop(remaining);

        let mut rows = self.rows.lock().unwrap();
        if rows
            .iter()
            .any(|(row, _)| row.request_id == commit.resolution.request_id)
        {
            return Err(PermissionsApplicationError::infrastructure(
                "sqlite",
                "request already resolved".to_string(),
            ));
        }
        let resolution = ApprovalResolution {
            id: commit.resolution.id.clone(),
            request_id: commit.resolution.request_id.clone(),
            principal_id: commit.resolution.principal_id.clone(),
            session_id: commit.resolution.session_id.clone(),
            generation_id: commit.resolution.generation_id.clone(),
            decision: commit.resolution.decision.clone(),
            state: commit.resolution.state,
            delivery_attempts: 0,
            last_error_code: None,
        };
        if let Some(intent) = &commit.grant_intent {
            self.grant_intents.lock().unwrap().push(PendingGrantIntent {
                id: intent.id.clone(),
                key: intent.key.clone(),
                effect: intent.effect,
                resolution_id: intent.resolution_id.clone(),
                now: intent.now.clone(),
            });
        }
        rows.push((resolution.clone(), commit.grant_intent.is_some()));
        self.journal
            .push(Event::Committed(resolution.id.as_str().to_string()));
        Ok(resolution)
    }

    fn find_by_request_id(
        &self,
        request_id: &str,
    ) -> Result<Option<ApprovalResolution>, PermissionsApplicationError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|(row, _)| row.request_id == request_id)
            .map(|(row, _)| row.clone()))
    }

    fn record_delivery_failure(
        &self,
        id: &ApprovalResolutionId,
        error_code: &str,
        _now: &str,
    ) -> Result<ApprovalResolution, PermissionsApplicationError> {
        let mut rows = self.rows.lock().unwrap();
        let (row, _) = rows
            .iter_mut()
            .find(|(row, _)| &row.id == id)
            .ok_or_else(|| PermissionsApplicationError::NotFound(id.as_str().to_string()))?;
        if !row.state.is_terminal() {
            row.state = ApprovalResolutionState::DeliveryFailed;
            row.delivery_attempts += 1;
            row.last_error_code = Some(error_code.to_string());
        }
        self.journal
            .push(Event::DeliveryFailed(id.as_str().to_string()));
        Ok(row.clone())
    }

    fn acknowledge_delivery_and_activate(
        &self,
        id: &ApprovalResolutionId,
        _now: &str,
    ) -> Result<ApprovalResolution, PermissionsApplicationError> {
        let mut rows = self.rows.lock().unwrap();
        let (row, has_grant) = rows
            .iter_mut()
            .find(|(row, _)| &row.id == id)
            .ok_or_else(|| PermissionsApplicationError::NotFound(id.as_str().to_string()))?;
        if !row.state.is_terminal() {
            row.state = ApprovalResolutionState::Delivered;
            if *has_grant {
                *self.grant_active.lock().unwrap() = true;
            }
        }
        self.journal
            .push(Event::Acknowledged(id.as_str().to_string()));
        Ok(row.clone())
    }

    fn mark_aborted_by_restart(&self, _now: &str) -> Result<usize, PermissionsApplicationError> {
        let mut reconciled = 0;
        for (row, _) in self.rows.lock().unwrap().iter_mut() {
            if row.state.needs_restart_reconciliation() {
                row.state = ApprovalResolutionState::AbortedByRestart;
                reconciled += 1;
            }
        }
        Ok(reconciled)
    }
}

/// What the waiter should do when the decision arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WaiterBehaviour {
    Applies,
    /// Reservation fails: the generation already ended.
    AlreadyGone,
    /// Reserved fine, then disappeared before the decision could be applied.
    VanishesBeforeDelivery,
    /// The transport itself failed.
    Errors,
}

struct FakeDelivery {
    journal: Arc<Journal>,
    behaviour: WaiterBehaviour,
    applied: Mutex<Vec<String>>,
}

impl FakeDelivery {
    fn new(journal: Arc<Journal>, behaviour: WaiterBehaviour) -> Self {
        Self {
            journal,
            behaviour,
            applied: Mutex::new(Vec::new()),
        }
    }
}

impl ApprovalDeliveryPort for FakeDelivery {
    fn reserve(
        &self,
        _request: &ApprovalRequest,
    ) -> Result<Option<DeliveryReservation>, PermissionsApplicationError> {
        if self.behaviour == WaiterBehaviour::AlreadyGone {
            return Ok(None);
        }
        self.journal.push(Event::Reserved);
        Ok(Some(DeliveryReservation {
            token: "reservation-1".to_string(),
        }))
    }

    fn deliver(
        &self,
        _reservation: &DeliveryReservation,
        _request: &ApprovalRequest,
        resolution_id: &ApprovalResolutionId,
        _effect: Effect,
    ) -> Result<DeliveryAcknowledgement, PermissionsApplicationError> {
        self.journal
            .push(Event::Delivered(resolution_id.as_str().to_string()));
        match self.behaviour {
            WaiterBehaviour::Errors => Err(PermissionsApplicationError::infrastructure(
                "agent_runtime",
                "waiter transport failed".to_string(),
            )),
            WaiterBehaviour::VanishesBeforeDelivery => Ok(DeliveryAcknowledgement::WaiterGone),
            WaiterBehaviour::AlreadyGone | WaiterBehaviour::Applies => {
                let mut applied = self.applied.lock().unwrap();
                // Exactly what a real waiter must do: one resolution id resumes execution once,
                // and a retry of the same id is acknowledged without resuming anything.
                if applied.contains(&resolution_id.as_str().to_string()) {
                    return Ok(DeliveryAcknowledgement::AlreadyApplied);
                }
                applied.push(resolution_id.as_str().to_string());
                Ok(DeliveryAcknowledgement::Applied)
            }
        }
    }
}

struct Fixture {
    use_case: ResolveApprovalUseCase,
    broker: ApprovalBroker,
    resolutions: Arc<FakeResolutions>,
    delivery: Arc<FakeDelivery>,
    journal: Arc<Journal>,
}

fn fixture_with(
    behaviour: WaiterBehaviour,
    resolutions: impl FnOnce(Arc<Journal>) -> FakeResolutions,
) -> Fixture {
    let journal = Arc::new(Journal::default());
    let clock: Arc<dyn PermissionsClockPort> = Arc::new(StepClock(Mutex::new(0)));
    let ids: Arc<dyn PermissionsIdPort> = Arc::new(SequentialIds::default());
    let broker = ApprovalBroker::new(
        Arc::new(FakePrincipals::default()),
        clock.clone(),
        ids.clone(),
        Arc::new(SilentEvents),
        60,
    );
    let resolutions = Arc::new(resolutions(journal.clone()));
    let delivery = Arc::new(FakeDelivery::new(journal.clone(), behaviour));
    Fixture {
        use_case: ResolveApprovalUseCase::new(
            broker.clone(),
            resolutions.clone(),
            delivery.clone(),
            clock,
            ids,
        ),
        broker,
        resolutions,
        delivery,
        journal,
    }
}

fn fixture(behaviour: WaiterBehaviour) -> Fixture {
    fixture_with(behaviour, FakeResolutions::new)
}

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

fn skill_provenance() -> SkillApprovalProvenance {
    SkillApprovalProvenance {
        parent_agent_id: "agent-1".to_string(),
        skill_id: "review".to_string(),
        tool_id: "check".to_string(),
        effective_revision: "a".repeat(64),
        source_scope: "global".to_string(),
        requested_capability: "tool:write_file".to_string(),
        delegated_operation: "write_file".to_string(),
        redacted_input_summary: "{}".to_string(),
        immutable_witness: "sha256:witness".to_string(),
    }
}

/// `permissions-approval`'s "Allow delivery is commit-before-effect".
///
/// The single most important assertion in this file. Not "the grant exists afterwards" — that is
/// true of the old flow too — but that the delivery event cannot appear before the commit event.
#[test]
fn no_decision_reaches_the_waiter_before_it_is_durable() {
    let fixture = fixture(WaiterBehaviour::Applies);
    let request = pending(&fixture.broker);

    let outcome = fixture
        .use_case
        .resolve(&request.id, ApprovalDecision::Approve, Scope::Session)
        .expect("resolve");

    assert!(outcome.reached_the_waiter());
    assert_eq!(
        fixture.journal.events(),
        vec![
            Event::Reserved,
            Event::Committed("resolution-1".to_string()),
            Event::Delivered("resolution-1".to_string()),
            Event::Acknowledged("resolution-1".to_string()),
        ]
    );
}

#[test]
fn a_delivered_approval_activates_its_remembered_grant() {
    let fixture = fixture(WaiterBehaviour::Applies);
    let request = pending(&fixture.broker);

    fixture
        .use_case
        .resolve(&request.id, ApprovalDecision::Approve, Scope::Session)
        .expect("resolve");

    assert!(fixture.resolutions.grant_is_active());
    assert_eq!(
        fixture.resolutions.state_of(&request.id),
        Some(ApprovalResolutionState::Delivered)
    );
    // The pending entry is released only after the outcome is known, never before.
    assert!(fixture.broker.get_pending(&request.id).is_none());
}

/// `permissions-approval`'s "Database fails before an approved action is delivered".
#[test]
fn a_commit_failure_delivers_nothing_and_leaves_the_approval_retryable() {
    let fixture = fixture_with(WaiterBehaviour::Applies, |journal| {
        FakeResolutions::failing_commits(journal, 1)
    });
    let request = pending(&fixture.broker);

    let failed = fixture
        .use_case
        .resolve(&request.id, ApprovalDecision::Approve, Scope::Session);

    assert!(failed.is_err());
    assert_eq!(fixture.journal.events(), vec![Event::Reserved]);
    assert!(fixture.delivery.applied.lock().unwrap().is_empty());
    assert!(!fixture.resolutions.grant_is_active());
    assert!(
        fixture.broker.get_pending(&request.id).is_some(),
        "a failure before commit consumed the pending request"
    );

    let retried = fixture
        .use_case
        .resolve(&request.id, ApprovalDecision::Approve, Scope::Session)
        .expect("retry");
    assert!(retried.reached_the_waiter());
}

/// `permissions-approval`'s "Stale generation is detected before commit".
#[test]
fn an_ended_generation_commits_evidence_without_delivering_or_granting() {
    let fixture = fixture(WaiterBehaviour::AlreadyGone);
    let request = pending(&fixture.broker);

    let outcome = fixture
        .use_case
        .resolve(&request.id, ApprovalDecision::Approve, Scope::Global)
        .expect("resolve");

    assert!(matches!(outcome, ResolveOutcome::StaleGeneration { .. }));
    assert_eq!(
        fixture.journal.events(),
        vec![Event::Committed("resolution-1".to_string())],
        "a stale resolution reserved or delivered something"
    );
    assert!(!fixture.resolutions.grant_is_active());
    assert!(fixture.resolutions.grant_intents.lock().unwrap().is_empty());
    assert_eq!(
        fixture.resolutions.state_of(&request.id),
        Some(ApprovalResolutionState::Stale)
    );
}

/// `permissions-approval`'s "Delivery fails after durable commit".
#[test]
fn a_delivery_failure_keeps_the_decision_durable_and_the_grant_inactive() {
    let fixture = fixture(WaiterBehaviour::Errors);
    let request = pending(&fixture.broker);

    let outcome = fixture
        .use_case
        .resolve(&request.id, ApprovalDecision::Approve, Scope::Session)
        .expect("resolve");

    assert_eq!(
        outcome,
        ResolveOutcome::DeliveryFailed {
            resolution_id: "resolution-1".to_string(),
            error_code: "delivery_failed",
        }
    );
    assert_eq!(
        fixture.resolutions.state_of(&request.id),
        Some(ApprovalResolutionState::DeliveryFailed)
    );
    assert!(
        !fixture.resolutions.grant_is_active(),
        "a grant became active for a decision nobody received"
    );
}

#[test]
fn a_waiter_that_vanishes_between_reservation_and_delivery_is_a_delivery_failure() {
    let fixture = fixture(WaiterBehaviour::VanishesBeforeDelivery);
    let request = pending(&fixture.broker);

    let outcome = fixture
        .use_case
        .resolve(&request.id, ApprovalDecision::Approve, Scope::Session)
        .expect("resolve");

    // Distinguished from a stale generation by its own reason code: one was never delivered
    // because nobody was there, the other because the attempt did not land.
    assert_eq!(
        outcome,
        ResolveOutcome::DeliveryFailed {
            resolution_id: "resolution-1".to_string(),
            error_code: "delivery_waiter_gone",
        }
    );
    assert!(!fixture.resolutions.grant_is_active());
}

/// `permissions-approval`'s "Two frontends resolve the same request concurrently".
#[test]
fn a_second_caller_gets_the_existing_state_and_writes_no_second_decision() {
    let fixture = fixture(WaiterBehaviour::Applies);
    let request = pending(&fixture.broker);
    // The interleaving stated rather than raced for: the first caller holds its claim.
    fixture.broker.claim(&request.id, "held-elsewhere");

    let outcome = fixture
        .use_case
        .resolve(&request.id, ApprovalDecision::Deny, Scope::Global)
        .expect("resolve");

    assert_eq!(
        outcome,
        ResolveOutcome::Resolving {
            resolution_id: "held-elsewhere".to_string()
        }
    );
    assert!(fixture.journal.events().is_empty());
    assert_eq!(fixture.resolutions.state_of(&request.id), None);
}

/// `permissions-approval`'s "Retry after an ambiguous response".
#[test]
fn a_retry_after_the_decision_landed_returns_that_decision_rather_than_a_second_one() {
    let fixture = fixture(WaiterBehaviour::Applies);
    let request = pending(&fixture.broker);
    fixture
        .use_case
        .resolve(&request.id, ApprovalDecision::Approve, Scope::Session)
        .expect("first resolve");

    let retried = fixture
        .use_case
        .resolve(&request.id, ApprovalDecision::Deny, Scope::Global)
        .expect("retry");

    assert_eq!(
        retried,
        ResolveOutcome::AlreadyResolved {
            resolution_id: "resolution-1".to_string(),
            state: ApprovalResolutionState::Delivered,
        }
    );
    // One delivery, one commit — the retry carried a *different* decision and changed nothing.
    assert_eq!(fixture.delivery.applied.lock().unwrap().len(), 1);
    assert_eq!(
        fixture
            .journal
            .events()
            .iter()
            .filter(|event| matches!(event, Event::Committed(_)))
            .count(),
        1
    );
}

#[test]
fn resolving_something_that_never_existed_is_distinguishable_from_a_retry() {
    let fixture = fixture(WaiterBehaviour::Applies);

    let outcome = fixture
        .use_case
        .resolve("never-existed", ApprovalDecision::Approve, Scope::Global)
        .expect("resolve");

    assert_eq!(outcome, ResolveOutcome::NotFound);
}

/// `permissions-approval`'s "Timeout storage failure remains an emergency fail-closed denial" —
/// the ordinary half: a timeout is the same single-winner flow, not a second path.
#[test]
fn a_timeout_denial_goes_through_the_same_single_winner_flow() {
    let fixture = fixture(WaiterBehaviour::Applies);
    let request = pending(&fixture.broker);

    let outcome = fixture
        .use_case
        .resolve_timed_out(&request.id)
        .expect("timeout resolve");

    assert_eq!(
        outcome,
        ResolveOutcome::Delivered {
            resolution_id: "resolution-1".to_string(),
            effect: Effect::Deny,
        }
    );
    // A timeout never remembers anything: it is the absence of a decision, not one.
    assert!(fixture.resolutions.grant_intents.lock().unwrap().is_empty());
    assert!(!fixture.resolutions.grant_is_active());
}

#[test]
fn a_timeout_racing_a_human_approval_cannot_produce_a_second_decision() {
    let fixture = fixture(WaiterBehaviour::Applies);
    let request = pending(&fixture.broker);
    fixture
        .use_case
        .resolve(&request.id, ApprovalDecision::Approve, Scope::Session)
        .expect("human resolves first");

    let swept = fixture
        .use_case
        .resolve_timed_out(&request.id)
        .expect("timeout sweep");

    assert_eq!(
        swept,
        ResolveOutcome::AlreadyResolved {
            resolution_id: "resolution-1".to_string(),
            state: ApprovalResolutionState::Delivered,
        }
    );
    assert_eq!(fixture.delivery.applied.lock().unwrap().len(), 1);
}

#[test]
fn a_skill_approval_can_never_produce_a_remembered_grant_through_this_path() {
    let fixture = fixture(WaiterBehaviour::Applies);
    let request = fixture
        .broker
        .create_skill_pending(
            skill_provenance(),
            Action::file_write(),
            Resource::file_path("src/lib.rs"),
            "session-1",
            "generation-1",
            "call-1",
            "project-1",
        )
        .expect("create skill pending");

    // Asks for the broadest scope there is; the forced Once is applied inside the use case.
    let outcome = fixture
        .use_case
        .resolve(&request.id, ApprovalDecision::Approve, Scope::Global)
        .expect("resolve");

    assert!(outcome.reached_the_waiter());
    assert!(fixture.resolutions.grant_intents.lock().unwrap().is_empty());
    assert!(!fixture.resolutions.grant_is_active());
}

#[test]
fn a_delegation_apply_approval_can_never_produce_a_remembered_grant_either() {
    let fixture = fixture(WaiterBehaviour::Applies);
    let request = fixture
        .broker
        .create_pending(
            "onepiece",
            Action::new("delegation.apply"),
            Resource::new("changeset/artifact-1"),
            "session-1",
            "generation-1",
            "call-1",
            "project-1",
        )
        .expect("create pending");

    fixture
        .use_case
        .resolve(&request.id, ApprovalDecision::Approve, Scope::Global)
        .expect("resolve");

    assert!(fixture.resolutions.grant_intents.lock().unwrap().is_empty());
}

#[test]
fn a_once_scoped_approval_is_delivered_and_remembers_nothing() {
    let fixture = fixture(WaiterBehaviour::Applies);
    let request = pending(&fixture.broker);

    let outcome = fixture
        .use_case
        .resolve(&request.id, ApprovalDecision::Approve, Scope::Once)
        .expect("resolve");

    assert!(outcome.reached_the_waiter());
    assert!(fixture.resolutions.grant_intents.lock().unwrap().is_empty());
}

#[test]
fn a_remembered_denial_is_written_as_an_intent_like_any_other_decision() {
    let fixture = fixture(WaiterBehaviour::Applies);
    let request = pending(&fixture.broker);

    fixture
        .use_case
        .resolve(&request.id, ApprovalDecision::Deny, Scope::Project)
        .expect("resolve");

    let intents = fixture.resolutions.grant_intents.lock().unwrap();
    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].effect, PersistedEffect::Deny);
    assert_eq!(
        intents[0].key.scope,
        RememberedScope::Project("project-1".to_string())
    );
    assert_eq!(
        intents[0].key.principal_id, request.principal_id,
        "the intent was bound to a different principal than the request"
    );
}

#[test]
fn the_grant_intent_written_by_a_commit_is_never_active_before_acknowledgement() {
    // Proven through the ledger fake rather than by inspecting a flag: the intent is handed to the
    // transaction, and only `acknowledge_delivery_and_activate` may turn it on.
    let fixture = fixture(WaiterBehaviour::Errors);
    let request = pending(&fixture.broker);

    fixture
        .use_case
        .resolve(&request.id, ApprovalDecision::Approve, Scope::Global)
        .expect("resolve");

    assert_eq!(fixture.resolutions.grant_intents.lock().unwrap().len(), 1);
    assert!(!fixture.resolutions.grant_is_active());
}

#[test]
fn the_ledger_stores_a_correlation_hash_and_never_the_providers_call_id() {
    let raw = "call-with-a-provider-chosen-body";
    let hashed = correlation_hash(raw);

    assert!(hashed.starts_with("fnv1a:"));
    assert!(!hashed.contains(raw));
    // Deterministic across runs and builds: correlation would silently break otherwise, and the
    // rows that broke would be the ones written before the upgrade.
    assert_eq!(hashed, correlation_hash(raw));
    assert_ne!(hashed, correlation_hash("call-2"));
}

#[test]
fn a_grant_intent_carries_the_resolution_that_gates_it() {
    let fixture = fixture(WaiterBehaviour::Applies);
    let request = pending(&fixture.broker);

    fixture
        .use_case
        .resolve(&request.id, ApprovalDecision::Approve, Scope::Session)
        .expect("resolve");

    let intents = fixture.resolutions.grant_intents.lock().unwrap();
    // Activation is addressed by resolution id alone: the acknowledgement arrives from the delivery
    // adapter, which knows the resolution and nothing about grant rows.
    assert_eq!(intents[0].resolution_id, "resolution-1");
    assert_eq!(
        intents[0].key.scope,
        RememberedScope::Session("session-1".to_string())
    );
}

#[test]
fn restart_reconciliation_never_revives_a_committed_but_unacknowledged_decision() {
    let fixture = fixture(WaiterBehaviour::Errors);
    let request = pending(&fixture.broker);
    fixture
        .use_case
        .resolve(&request.id, ApprovalDecision::Approve, Scope::Session)
        .expect("resolve");

    let reconciled = fixture
        .resolutions
        .mark_aborted_by_restart("100")
        .expect("reconcile");

    assert_eq!(reconciled, 1);
    assert_eq!(
        fixture.resolutions.state_of(&request.id),
        Some(ApprovalResolutionState::AbortedByRestart)
    );
    assert!(
        !fixture.resolutions.grant_is_active(),
        "a restart activated a grant whose delivery was never acknowledged"
    );
    // And no pending request is recreated for a new generation to inherit.
    assert!(fixture.broker.get_pending(&request.id).is_none());
}

#[test]
fn one_resolution_id_resumes_the_waiter_at_most_once() {
    let fixture = fixture(WaiterBehaviour::Applies);
    let request = pending(&fixture.broker);
    let resolution_id = ApprovalResolutionId::parse("resolution-manual").expect("id");
    let reservation = fixture
        .delivery
        .reserve(&request)
        .expect("reserve")
        .expect("a live waiter");

    let first = fixture
        .delivery
        .deliver(&reservation, &request, &resolution_id, Effect::Allow)
        .expect("first delivery");
    let second = fixture
        .delivery
        .deliver(&reservation, &request, &resolution_id, Effect::Allow)
        .expect("retried delivery");

    assert_eq!(first, DeliveryAcknowledgement::Applied);
    assert_eq!(
        second,
        DeliveryAcknowledgement::AlreadyApplied,
        "a retried delivery released a second execution"
    );
}
