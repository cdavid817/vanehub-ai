use super::*;
use crate::contexts::skill_evolution_curation::domain::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct FakeStore {
    binding: CuratorApplicationBinding,
    duplicate: bool,
    application_status: CuratorApplicationStatus,
    pending: Vec<CuratorPreparedApplication>,
    prepared_intent: Option<CuratorApplicationIntent>,
    finalizations: Vec<Result<CuratorOverlayApplicationReceipt, CuratorApplicationFailure>>,
    fail_finalize: bool,
    events: Rc<RefCell<Vec<&'static str>>>,
}

impl CuratorApplicationStore for FakeStore {
    fn existing_application(
        &mut self,
        application_id: &str,
        _: &str,
        _: u64,
        _: &str,
        _: &str,
        _: Option<&CuratorSystemPolicyAuthorizationV1>,
    ) -> Result<Option<CuratorPreparedApplication>, CuratorApplicationStoreError> {
        Ok(self
            .duplicate
            .then(|| prepared(&self.binding, application_id, self.application_status, true)))
    }

    fn application_binding(
        &mut self,
        _: &str,
    ) -> Result<CuratorApplicationBinding, CuratorApplicationStoreError> {
        Ok(self.binding.clone())
    }

    fn prepare_application_intent(
        &mut self,
        intent: &CuratorApplicationIntent,
    ) -> Result<CuratorPreparedApplication, CuratorApplicationStoreError> {
        self.events.borrow_mut().push("intent");
        self.prepared_intent = Some(intent.clone());
        Ok(prepared(
            &self.binding,
            &intent.application_id,
            self.application_status,
            self.duplicate,
        ))
    }

    fn finalize_application(
        &mut self,
        application_id: &str,
        _: u64,
        result: Result<&CuratorOverlayApplicationReceipt, CuratorApplicationFailure>,
        _: i64,
    ) -> Result<CuratorApplication, CuratorApplicationStoreError> {
        self.events.borrow_mut().push("finalize");
        if self.fail_finalize {
            return Err(CuratorApplicationStoreError::Storage);
        }
        let owned = result.cloned();
        self.finalizations.push(owned.clone());
        let (status, overlay_revision, overlay_history_id, failure_code) = match owned {
            Ok(receipt) => (
                if receipt.duplicate {
                    CuratorApplicationStatus::Reconciled
                } else {
                    CuratorApplicationStatus::Applied
                },
                Some(receipt.overlay_revision),
                Some(receipt.overlay_history_id),
                None,
            ),
            Err(failure) => (
                CuratorApplicationStatus::Failed,
                None,
                None,
                Some(failure.code().to_string()),
            ),
        };
        Ok(application(
            application_id,
            status,
            2,
            overlay_revision,
            overlay_history_id,
            failure_code,
        ))
    }

    fn pending_applications(
        &mut self,
        _: usize,
    ) -> Result<Vec<CuratorPreparedApplication>, CuratorApplicationStoreError> {
        Ok(self.pending.clone())
    }

    fn prepare_failed_retry(
        &mut self,
        _: &str,
        expected_candidate_revision: u64,
        _: i64,
    ) -> Result<u64, CuratorApplicationStoreError> {
        Ok(expected_candidate_revision + 1)
    }
}

struct FakeOverlay {
    apply_result: RefCell<Result<CuratorOverlayApplicationReceipt, CuratorApplicationFailure>>,
    find_result:
        RefCell<Result<Option<CuratorOverlayApplicationReceipt>, CuratorApplicationFailure>>,
    apply_calls: Cell<usize>,
    find_calls: Cell<usize>,
    requests: RefCell<Vec<CuratorOverlayApplicationRequest>>,
    events: Rc<RefCell<Vec<&'static str>>>,
}

impl CuratorOverlayApplicationPort for FakeOverlay {
    fn apply(
        &self,
        request: &CuratorOverlayApplicationRequest,
    ) -> Result<CuratorOverlayApplicationReceipt, CuratorApplicationFailure> {
        self.events.borrow_mut().push("overlay");
        self.apply_calls.set(self.apply_calls.get() + 1);
        self.requests.borrow_mut().push(request.clone());
        self.apply_result.borrow().clone()
    }

    fn find_committed(
        &self,
        request: &CuratorOverlayApplicationRequest,
    ) -> Result<Option<CuratorOverlayApplicationReceipt>, CuratorApplicationFailure> {
        self.find_calls.set(self.find_calls.get() + 1);
        self.requests.borrow_mut().push(request.clone());
        self.find_result.borrow().clone()
    }
}

fn binding() -> CuratorApplicationBinding {
    CuratorApplicationBinding {
        decision: CuratorDecisionBinding {
            candidate_id: "candidate-1".into(),
            candidate_revision: 6,
            candidate_hash: "candidate-hash".into(),
            policy_hash: "policy-hash".into(),
            maximum_defer_days: 180,
            state: CuratorCandidateState::ReadyForReview,
            staleness: vec![],
            ready_draft: Some(CuratorReadyDraftWitness {
                draft_revision: 2,
                assessment_id: "assessment-1".into(),
            }),
            current_preview: Some(CuratorApprovalPreviewWitness {
                preview_id: "preview-1".into(),
                witness_hash: "preview-hash".into(),
                effective_diff_hash: "diff-projection-hash".into(),
                draft_revision: 2,
                assessment_id: "assessment-1".into(),
                issued_at_ms: 1_000,
                expires_at_ms: 10_000,
                diffs_complete: true,
                validation_complete: true,
            }),
        },
        workspace_id: "project:/workspace".into(),
        target_skill_id: "skill-1".into(),
        overlay_scope: "project".into(),
        mutation: CuratorDraftMutationInput::LearnedGuidance {
            guidance: "Keep exact witnesses".into(),
        },
        overlay_witnesses: CuratorApplicationOverlayWitnesses {
            expected_overlay_revision: Some(4),
            base_instruction_hash: "base-instruction".into(),
            base_package_hash: "base-package".into(),
            proposed_effective_hash: "effective-content-hash".into(),
            expected_pinned: false,
        },
    }
}

fn application(
    application_id: &str,
    status: CuratorApplicationStatus,
    revision: u64,
    overlay_revision: Option<String>,
    overlay_history_id: Option<String>,
    failure_code: Option<String>,
) -> CuratorApplication {
    CuratorApplication {
        application_id: application_id.into(),
        candidate_id: "candidate-1".into(),
        decision_id: "decision-1".into(),
        status,
        approved_witness_hash: "preview-hash".into(),
        overlay_revision,
        overlay_history_id,
        failure_code,
        revision,
    }
}

fn prepared(
    binding: &CuratorApplicationBinding,
    application_id: &str,
    status: CuratorApplicationStatus,
    duplicate: bool,
) -> CuratorPreparedApplication {
    CuratorPreparedApplication {
        application: application(application_id, status, 1, None, None, None),
        binding: binding.clone(),
        duplicate,
    }
}

fn receipt(duplicate: bool) -> CuratorOverlayApplicationReceipt {
    CuratorOverlayApplicationReceipt {
        overlay_revision: "5".into(),
        overlay_history_id: "history-5".into(),
        effective_diff_hash: "effective-content-hash".into(),
        duplicate,
    }
}

fn fixture() -> (FakeStore, FakeOverlay, Rc<RefCell<Vec<&'static str>>>) {
    let events = Rc::new(RefCell::new(vec![]));
    (
        FakeStore {
            binding: binding(),
            duplicate: false,
            application_status: CuratorApplicationStatus::IntentRecorded,
            pending: vec![],
            prepared_intent: None,
            finalizations: vec![],
            fail_finalize: false,
            events: Rc::clone(&events),
        },
        FakeOverlay {
            apply_result: RefCell::new(Ok(receipt(false))),
            find_result: RefCell::new(Ok(None)),
            apply_calls: Cell::new(0),
            find_calls: Cell::new(0),
            requests: RefCell::new(vec![]),
            events: Rc::clone(&events),
        },
        events,
    )
}

fn approval() -> CuratorApprovalRequest<'static> {
    CuratorApprovalRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 6,
        confirmed_preview_hash: "preview-hash",
        confirmed_effective_diff_hash: "diff-projection-hash",
        idempotency_key: "approval-1",
    }
}

fn system_authorization() -> CuratorSystemPolicyAuthorizationV1 {
    CuratorSystemPolicyAuthorizationV1 {
        run_id: "run-1".into(),
        eligibility_id: "eligibility-1".into(),
        eligibility_proof_hash: "eligibility-proof".into(),
        preflight_witness_hash: "preflight-proof".into(),
        policy_witness_hash: "policy-proof".into(),
        rate_reservation_id: "reservation-1".into(),
        authorized_at_ms: 2_000,
    }
}

fn system_request(
    authorization: &CuratorSystemPolicyAuthorizationV1,
) -> CuratorSystemPolicyApplicationRequest<'_> {
    CuratorSystemPolicyApplicationRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 6,
        preview_hash: "preview-hash",
        effective_diff_hash: "diff-projection-hash",
        idempotency_key: "auto-application-1",
        authorization,
    }
}

#[test]
fn system_policy_uses_distinct_authorization_without_interactive_approval() {
    let (mut store, overlay, events) = fixture();
    let authorization = system_authorization();
    let outcome =
        CuratorApplicationService::new(&mut store, &overlay, CuratorTrustedActor::system(2_000))
            .apply_system_policy(system_request(&authorization))
            .expect("system policy application");

    assert!(matches!(outcome, CuratorApplicationOutcome::Applied(_)));
    assert_eq!(&*events.borrow(), &["intent", "overlay", "finalize"]);
    let intent = store.prepared_intent.as_ref().expect("intent");
    assert_eq!(intent.decision.actor_class, CuratorActorClass::System);
    assert_eq!(intent.decision.reason_code, "system_policy_authorized");
    assert_eq!(
        intent.system_policy_authorization.as_ref(),
        Some(&authorization)
    );
}

#[test]
fn system_policy_rejects_interactive_actor_and_forbidden_patch_mutation() {
    let authorization = system_authorization();
    let (mut interactive_store, interactive_overlay, _) = fixture();
    let interactive = CuratorApplicationService::new(
        &mut interactive_store,
        &interactive_overlay,
        CuratorTrustedActor::local_interactive_user(2_000),
    )
    .apply_system_policy(system_request(&authorization));
    assert_eq!(
        interactive,
        Err(CuratorApplicationServiceError::Unauthorized)
    );
    assert!(interactive_store.prepared_intent.is_none());

    let (mut patch_store, patch_overlay, _) = fixture();
    patch_store.binding.mutation = CuratorDraftMutationInput::ExactPatch {
        old_string: "old".into(),
        new_string: "new".into(),
        replace_all: false,
    };
    let patch = CuratorApplicationService::new(
        &mut patch_store,
        &patch_overlay,
        CuratorTrustedActor::system(2_000),
    )
    .apply_system_policy(system_request(&authorization));
    assert_eq!(
        patch,
        Err(CuratorApplicationServiceError::InvalidInput(
            "system_policy_mutation_forbidden"
        ))
    );
    assert!(patch_store.prepared_intent.is_none());
}

#[test]
fn durable_intent_precedes_exact_overlay_commit_and_finalization() {
    let (mut store, overlay, events) = fixture();
    let outcome = CuratorApplicationService::new(
        &mut store,
        &overlay,
        CuratorTrustedActor::local_interactive_user(2_000),
    )
    .approve(approval())
    .expect("approve");

    assert!(matches!(outcome, CuratorApplicationOutcome::Applied(_)));
    assert_eq!(&*events.borrow(), &["intent", "overlay", "finalize"]);
    let intent = store.prepared_intent.as_ref().expect("intent");
    assert_eq!(intent.approved_diff_hash, "diff-projection-hash");
    assert_eq!(intent.expected_effective_hash, "effective-content-hash");
    assert_eq!(
        overlay.requests.borrow()[0]
            .witnesses
            .expected_overlay_revision,
        Some(4)
    );
}

#[test]
fn overlay_failure_is_terminalized_without_rewriting_the_failure() {
    let (mut store, overlay, _) = fixture();
    *overlay.apply_result.borrow_mut() = Err(CuratorApplicationFailure::Pinned);
    let outcome = CuratorApplicationService::new(
        &mut store,
        &overlay,
        CuratorTrustedActor::local_interactive_user(2_000),
    )
    .approve(approval())
    .expect("terminal failure");

    let CuratorApplicationOutcome::Failed(application) = outcome else {
        panic!("expected failure");
    };
    assert_eq!(application.failure_code.as_deref(), Some("overlay_pinned"));
    assert_eq!(overlay.apply_calls.get(), 1);
}

#[test]
fn duplicate_pending_request_never_replays_overlay() {
    let (mut store, overlay, _) = fixture();
    store.duplicate = true;
    let result = CuratorApplicationService::new(
        &mut store,
        &overlay,
        CuratorTrustedActor::local_interactive_user(2_000),
    )
    .approve(approval());

    assert_eq!(
        result,
        Err(CuratorApplicationServiceError::RecoveryRequired)
    );
    assert!(store.prepared_intent.is_none());
    assert_eq!(overlay.apply_calls.get(), 0);
    assert_eq!(overlay.find_calls.get(), 1);
}

#[test]
fn recovery_reconciles_history_but_query_failure_leaves_intent_pending() {
    let (mut store, overlay, _) = fixture();
    let pending = prepared(
        &store.binding,
        "application-1",
        CuratorApplicationStatus::IntentRecorded,
        true,
    );
    store.pending = vec![pending.clone()];
    *overlay.find_result.borrow_mut() = Ok(Some(receipt(true)));
    let outcomes =
        CuratorApplicationService::new(&mut store, &overlay, CuratorTrustedActor::system(3_000))
            .recover_pending()
            .expect("recover");
    assert!(matches!(outcomes[0], CuratorApplicationOutcome::Applied(_)));
    assert_eq!(overlay.apply_calls.get(), 0);
    assert_eq!(overlay.find_calls.get(), 1);
    assert_eq!(store.finalizations.len(), 1);

    store.pending = vec![pending];
    store.finalizations.clear();
    *overlay.find_result.borrow_mut() = Err(CuratorApplicationFailure::Integrity);
    let error =
        CuratorApplicationService::new(&mut store, &overlay, CuratorTrustedActor::system(3_001))
            .recover_pending();
    assert_eq!(
        error,
        Err(CuratorApplicationServiceError::Overlay(
            CuratorApplicationFailure::Integrity
        ))
    );
    assert!(store.finalizations.is_empty());
}

#[test]
fn overlay_commit_survives_sqlite_finalization_failure_and_is_reconciled() {
    let (mut store, overlay, _) = fixture();
    store.fail_finalize = true;
    let result = CuratorApplicationService::new(
        &mut store,
        &overlay,
        CuratorTrustedActor::local_interactive_user(2_000),
    )
    .approve(approval());
    assert_eq!(
        result,
        Err(CuratorApplicationServiceError::Store(
            CuratorApplicationStoreError::Storage
        ))
    );
    assert_eq!(overlay.apply_calls.get(), 1);

    store.fail_finalize = false;
    let application_id = store
        .prepared_intent
        .as_ref()
        .expect("intent")
        .application_id
        .clone();
    store.pending = vec![prepared(
        &store.binding,
        &application_id,
        CuratorApplicationStatus::IntentRecorded,
        true,
    )];
    *overlay.find_result.borrow_mut() = Ok(Some(receipt(true)));
    let recovered =
        CuratorApplicationService::new(&mut store, &overlay, CuratorTrustedActor::system(3_000))
            .recover_pending()
            .expect("reconcile");
    assert!(matches!(
        recovered[0],
        CuratorApplicationOutcome::Applied(_)
    ));
    assert_eq!(overlay.apply_calls.get(), 1);
}
