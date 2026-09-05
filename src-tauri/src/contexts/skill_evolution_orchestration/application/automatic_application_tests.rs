use super::*;
use crate::contexts::skill_evolution_orchestration::domain::{
    AutoApplyProbationV1, AutomaticEvolutionApplicationV1, AutomaticPreflightWitnessV1,
};
use std::cell::{Cell, RefCell};

struct Preflight {
    witness: AutomaticPreflightWitnessV1,
    calls: Cell<usize>,
}

impl AutomaticPreflightConsumptionPort for Preflight {
    fn consume_or_recover(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<AutomaticPreflightWitnessV1, AutomaticApplicationError> {
        self.calls.set(self.calls.get() + 1);
        Ok(self.witness.clone())
    }
}

struct Curator {
    receipt: RefCell<SystemPolicyCuratorReceiptV1>,
    requests: RefCell<Vec<SystemPolicyCuratorRequestV1>>,
}

impl SystemPolicyCuratorPort for Curator {
    fn apply(
        &self,
        request: SystemPolicyCuratorRequestV1,
    ) -> Result<SystemPolicyCuratorReceiptV1, AutomaticApplicationError> {
        self.requests.borrow_mut().push(request);
        Ok(self.receipt.borrow().clone())
    }
}

#[derive(Default)]
struct Finalizer {
    applications: RefCell<Vec<AutomaticEvolutionApplicationV1>>,
    probations: RefCell<Vec<AutoApplyProbationV1>>,
}

impl AutomaticApplicationFinalizationPort for Finalizer {
    fn finalize(
        &self,
        application: &AutomaticEvolutionApplicationV1,
        probation: &AutoApplyProbationV1,
        _: &str,
        _: u64,
    ) -> Result<bool, AutomaticApplicationError> {
        self.applications.borrow_mut().push(application.clone());
        self.probations.borrow_mut().push(probation.clone());
        Ok(self.applications.borrow().len() == 1)
    }
}

fn fixture() -> (Preflight, Curator, Finalizer) {
    (
        Preflight {
            witness: AutomaticPreflightWitnessV1 {
                witness_id: "preflight-1".into(),
                run_id: "run-1".into(),
                eligibility_id: "eligibility-1".into(),
                eligibility_proof_hash: "eligibility-proof".into(),
                reservation_id: "reservation-1".into(),
                overlay_preview_hash: "preview-proof".into(),
                proof_hash: "preflight-proof".into(),
                issued_at_ms: 1,
                expires_at_ms: 5_001,
                revision: 1,
            },
            calls: Cell::new(0),
        },
        Curator {
            receipt: RefCell::new(SystemPolicyCuratorReceiptV1 {
                application_id: "application-1".into(),
                applied: true,
                overlay_revision: Some("2".into()),
                overlay_history_id: Some("history-1".into()),
                failure_code: None,
            }),
            requests: RefCell::new(vec![]),
        },
        Finalizer::default(),
    )
}

fn command() -> AutomaticApplicationCommandV1 {
    AutomaticApplicationCommandV1 {
        preflight_witness_id: "preflight-1".into(),
        preflight_proof_hash: "preflight-proof".into(),
        current_overlay_preview_hash: "preview-proof".into(),
        candidate_id: "candidate-1".into(),
        expected_candidate_revision: 2,
        effective_diff_hash: "diff-proof".into(),
        policy_witness_hash: "policy-proof".into(),
        run_item_id: "item-1".into(),
        expected_rate_revision: 0,
        workspace_id: "workspace-1".into(),
        target_skill_id: "skill-1".into(),
        prior_effective_hash: "prior-proof".into(),
        resulting_effective_hash: "result-proof".into(),
        evidence_fingerprint: "fingerprint-1".into(),
        evidence_categories: vec!["verified_correction".into()],
        baseline_witness_hash: "baseline-proof".into(),
        now_ms: 10,
    }
}

#[test]
fn coordinator_uses_one_provenance_chain_and_is_recovery_safe() {
    let (preflight, curator, finalizer) = fixture();
    let coordinator = AutomaticApplicationCoordinator::new(&preflight, &curator, &finalizer);
    let first = coordinator.apply(command()).expect("first application");
    let recovered = coordinator.apply(command()).expect("recovered application");

    assert_eq!(first, recovered);
    assert_eq!(first.application_id, "application-1");
    assert_eq!(first.curator_application_id, first.application_id);
    assert_eq!(first.overlay_application_id, first.application_id);
    assert_eq!(
        curator.requests.borrow()[0].idempotency_key,
        "reservation-1"
    );
    assert_eq!(
        finalizer.probations.borrow()[0].application_id,
        first.application_id
    );
}

#[test]
fn failed_curator_application_never_creates_local_success() {
    let (preflight, curator, finalizer) = fixture();
    curator.receipt.borrow_mut().applied = false;
    curator.receipt.borrow_mut().failure_code = Some("overlay_pinned".into());
    let result =
        AutomaticApplicationCoordinator::new(&preflight, &curator, &finalizer).apply(command());

    assert_eq!(result, Err(AutomaticApplicationError::CuratorFailed));
    assert!(finalizer.applications.borrow().is_empty());
}
