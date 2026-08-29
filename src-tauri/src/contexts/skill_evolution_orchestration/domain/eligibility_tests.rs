use super::*;

type EligibilityFailure = (&'static str, fn(&mut AutoEligibilityInputV1));

fn input() -> AutoEligibilityInputV1 {
    AutoEligibilityInputV1 {
        eligibility_id: "eligibility-one".into(),
        run_id: "run-one".into(),
        draft_id: "draft-one".into(),
        target_skill_id: "skill-one".into(),
        mode: EvolutionPolicyMode::Enabled,
        consent_current: true,
        skill_allowlisted: true,
        stable_skill_id: true,
        assessment_current: true,
        assessment_route_advance: true,
        assessment_deterministic: true,
        target_clear: true,
        target_attribution: AutomaticTargetAttributionV1::Verified,
        target_model_consulted: false,
        confidence_basis_points: 9_500,
        risk_low: true,
        quality_checks: AUTO_DRAFT_QUALITY_CHECK_ORDER_V1
            .iter()
            .map(|code| ((*code).into(), "pass".into()))
            .collect(),
        independent_supporting_runs: 3,
        authorized_verified_correction: true,
        independent_confirmations: 2,
        draft_current: true,
        draft_trusted: true,
        draft_provenance: AutomaticDraftProvenance::DeterministicAuthorizedCorrection,
        mutation_kind: AutomaticMutationKindV1::LearnedGuidance,
        target_lifecycle: AutomaticTargetLifecycleV1::ActiveMutable,
        target_pinned: false,
        overlay_healthy: true,
        overlay_trusted: true,
        overlay_scope_system: false,
        idle_capacity: true,
        rate_capacity: true,
        circuit_breakers_closed: true,
        evaluated_at_ms: 10,
        revision: 0,
    }
}

#[test]
fn every_condition_produces_one_ordered_witness_and_all_must_pass() {
    let baseline = evaluate_auto_eligibility(&input()).expect("eligible");
    assert_eq!(baseline.result, AutoEligibilityResult::Eligible);
    assert_eq!(baseline.predicates.len(), 26);
    assert!(baseline.predicates.iter().all(|predicate| {
        predicate.passed && predicate.safe_reason_code.is_none() && predicate.witness_hash.is_some()
    }));

    let failures: [EligibilityFailure; 25] = [
        ("consent_current", |v| v.consent_current = false),
        ("skill_allowlisted", |v| v.skill_allowlisted = false),
        ("stable_skill_id", |v| v.stable_skill_id = false),
        ("assessment_current", |v| v.assessment_current = false),
        ("assessment_advance", |v| v.assessment_route_advance = false),
        ("assessment_deterministic", |v| {
            v.assessment_deterministic = false
        }),
        ("target_clear", |v| v.target_clear = false),
        ("target_attribution_verified", |v| {
            v.target_attribution = AutomaticTargetAttributionV1::Correlated;
        }),
        ("target_model_independent", |v| {
            v.target_model_consulted = true
        }),
        ("confidence_threshold", |v| {
            v.confidence_basis_points = 9_499
        }),
        ("risk_low", |v| v.risk_low = false),
        ("nine_quality_checks_pass", |v| {
            v.quality_checks[8].1 = "review".into()
        }),
        ("independent_support", |v| {
            v.independent_supporting_runs = 2;
            v.authorized_verified_correction = false;
        }),
        ("draft_current", |v| v.draft_current = false),
        ("draft_trusted", |v| v.draft_trusted = false),
        ("draft_provenance", |v| {
            v.draft_provenance = AutomaticDraftProvenance::ModelGenerated;
        }),
        ("learned_guidance_only", |v| {
            v.mutation_kind = AutomaticMutationKindV1::ExactPatch;
        }),
        ("target_mutable_active", |v| {
            v.target_lifecycle = AutomaticTargetLifecycleV1::Archived;
        }),
        ("target_unpinned", |v| v.target_pinned = true),
        ("overlay_healthy", |v| v.overlay_healthy = false),
        ("overlay_trusted", |v| v.overlay_trusted = false),
        ("overlay_scope_bounded", |v| v.overlay_scope_system = true),
        ("idle_capacity", |v| v.idle_capacity = false),
        ("rate_capacity", |v| v.rate_capacity = false),
        ("circuit_breakers_closed", |v| {
            v.circuit_breakers_closed = false
        }),
    ];
    for (condition, fail) in failures {
        let mut candidate = input();
        candidate.confidence_basis_points = 10_000;
        candidate.independent_supporting_runs = u8::MAX;
        fail(&mut candidate);
        let proof = evaluate_auto_eligibility(&candidate).expect("proof");
        assert_ne!(proof.result, AutoEligibilityResult::Eligible, "{condition}");
        let failed = proof
            .predicates
            .iter()
            .find(|predicate| predicate.condition == condition)
            .expect("named predicate");
        assert!(!failed.passed, "{condition}");
        let expected_reason = format!("auto.{condition}.failed");
        assert_eq!(
            failed.safe_reason_code.as_deref(),
            Some(expected_reason.as_str())
        );
    }
}

#[test]
fn observe_mode_records_would_apply_without_creating_mutation_authority() {
    let mut candidate = input();
    candidate.mode = EvolutionPolicyMode::Observe;
    let proof = evaluate_auto_eligibility(&candidate).expect("observe proof");
    assert_eq!(proof.result, AutoEligibilityResult::WouldApply);
    assert!(proof.overlay_preview_hash.is_none());
    assert!(proof.predicates.iter().all(|predicate| predicate.passed));

    candidate.mode = EvolutionPolicyMode::Off;
    assert_eq!(
        evaluate_auto_eligibility(&candidate)
            .expect("off proof")
            .result,
        AutoEligibilityResult::Ineligible
    );
}

#[test]
fn support_threshold_accepts_only_the_two_specified_paths() {
    let mut candidate = input();
    candidate.independent_supporting_runs = 0;
    assert_eq!(
        evaluate_auto_eligibility(&candidate)
            .expect("authorized path")
            .result,
        AutoEligibilityResult::Eligible
    );
    candidate.independent_confirmations = 1;
    assert_eq!(
        evaluate_auto_eligibility(&candidate)
            .expect("insufficient confirmation")
            .result,
        AutoEligibilityResult::RoutedToCurator
    );
    candidate.independent_supporting_runs = 3;
    candidate.authorized_verified_correction = false;
    assert_eq!(
        evaluate_auto_eligibility(&candidate)
            .expect("three run path")
            .result,
        AutoEligibilityResult::Eligible
    );
}

#[test]
fn every_permanently_excluded_mutation_and_provenance_routes_to_curator() {
    for mutation in [
        AutomaticMutationKindV1::ExactPatch,
        AutomaticMutationKindV1::File,
        AutomaticMutationKindV1::Script,
        AutomaticMutationKindV1::ToolDefinition,
        AutomaticMutationKindV1::Command,
        AutomaticMutationKindV1::PermissionExpansion,
        AutomaticMutationKindV1::SideEffectExpansion,
        AutomaticMutationKindV1::Unknown,
    ] {
        let mut candidate = input();
        candidate.mutation_kind = mutation;
        assert_eq!(
            evaluate_auto_eligibility(&candidate)
                .expect("mutation exclusion")
                .result,
            AutoEligibilityResult::RoutedToCurator,
            "{mutation:?}"
        );
    }
    for provenance in [
        AutomaticDraftProvenance::UserAuthored,
        AutomaticDraftProvenance::Edited,
        AutomaticDraftProvenance::ModelGenerated,
        AutomaticDraftProvenance::Imported,
        AutomaticDraftProvenance::ExactPatch,
        AutomaticDraftProvenance::File,
        AutomaticDraftProvenance::Script,
        AutomaticDraftProvenance::Unknown,
    ] {
        let mut candidate = input();
        candidate.draft_provenance = provenance;
        assert_eq!(
            evaluate_auto_eligibility(&candidate)
                .expect("provenance exclusion")
                .result,
            AutoEligibilityResult::RoutedToCurator,
            "{provenance:?}"
        );
    }
}

#[test]
fn transient_capacity_failures_wait_instead_of_mutating_or_claiming_success() {
    let failures: [fn(&mut AutoEligibilityInputV1); 2] = [
        |value: &mut AutoEligibilityInputV1| value.idle_capacity = false,
        |value: &mut AutoEligibilityInputV1| value.rate_capacity = false,
    ];
    for fail in failures {
        let mut candidate = input();
        fail(&mut candidate);
        assert_eq!(
            evaluate_auto_eligibility(&candidate)
                .expect("waiting proof")
                .result,
            AutoEligibilityResult::Waiting
        );
    }
}
