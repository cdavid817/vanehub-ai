use super::*;

fn probation() -> AutoApplyProbationV1 {
    AutoApplyProbationV1 {
        probation_id: "probation-1".into(),
        application_id: "application-1".into(),
        workspace_id: "workspace-1".into(),
        skill_id: "skill-1".into(),
        status: ProbationStatus::Active,
        prior_effective_hash: "prior".into(),
        current_effective_hash: "current".into(),
        evidence_fingerprint: "fingerprint".into(),
        evidence_categories: vec!["verification".into()],
        baseline_witness_hash: "baseline".into(),
        starts_at_ms: 10,
        ends_at_ms: 20,
        revision: 0,
    }
}

fn observation(id: &str, source: &str) -> ProbationObservationV1 {
    ProbationObservationV1 {
        observation_id: id.into(),
        probation_id: "probation-1".into(),
        source_kind: "verification".into(),
        source_id: source.into(),
        source_revision: 1,
        verified: true,
        negative: true,
        baseline_exceeded: true,
        harmful_correction: false,
        safe_category: "verification".into(),
        witness_hash: format!("witness-{id}"),
        observed_at_ms: 15,
    }
}

#[test]
fn two_independent_compatible_negatives_are_required() {
    let one = observation("one", "same-source");
    let duplicate_source = observation("two", "same-source");
    assert_eq!(
        evaluate_probation(&probation(), &[one.clone(), duplicate_source], 16),
        Ok(ProbationEvaluation::Active)
    );
    assert_eq!(
        evaluate_probation(&probation(), &[one, observation("two", "other-source")], 16),
        Ok(ProbationEvaluation::Regressed)
    );
}

#[test]
fn explicit_harmful_correction_regresses_once_but_unrelated_participation_does_not() {
    let mut harmful = observation("harmful", "feedback-1");
    harmful.source_kind = "explicit_correction".into();
    harmful.safe_category = "unrelated".into();
    harmful.harmful_correction = true;
    assert_eq!(
        evaluate_probation(&probation(), &[harmful], 16),
        Ok(ProbationEvaluation::Regressed)
    );
    let mut unrelated = observation("unrelated", "run-2");
    unrelated.safe_category = "other-task".into();
    assert_eq!(
        evaluate_probation(&probation(), &[unrelated], 21),
        Ok(ProbationEvaluation::Healthy)
    );
}
