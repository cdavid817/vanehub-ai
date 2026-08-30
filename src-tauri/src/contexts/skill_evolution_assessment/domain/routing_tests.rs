use super::*;

#[test]
fn confidence_uses_bounded_versioned_components() {
    let checks = passing_checks();
    let assessment = calculate_confidence(&ConfidenceInput {
        verified_corrected_feedback: true,
        independent_nonduplicate_runs: 1,
        selection_score: 100,
        selection_margin: 15,
        independent_lineages: 2,
        checks: &checks,
        material_contradiction: false,
        model: Some(ModelCorroboration {
            confidence: 8.0,
            citations_valid: true,
        }),
    });

    assert_eq!(assessment.level, AssessmentConfidence::High);
    assert_eq!(assessment.breakdown.policy_version, CONFIDENCE_POLICY_V1);
    assert_eq!(assessment.breakdown.model_corroboration_bps, 500);
    assert_eq!(assessment.breakdown.system_confidence_bps, 10_000);
}

#[test]
fn invalid_model_citations_add_no_confidence_and_contradictions_penalize() {
    let checks = passing_checks();
    let assessment = calculate_confidence(&ConfidenceInput {
        verified_corrected_feedback: true,
        independent_nonduplicate_runs: 2,
        selection_score: 100,
        selection_margin: 15,
        independent_lineages: 2,
        checks: &checks,
        material_contradiction: true,
        model: Some(ModelCorroboration {
            confidence: 1.0,
            citations_valid: false,
        }),
    });

    assert_eq!(assessment.breakdown.model_corroboration_bps, 0);
    assert_eq!(assessment.breakdown.contradiction_penalty_bps, 2_000);
    assert_eq!(assessment.breakdown.system_confidence_bps, 7_500);
    assert_eq!(assessment.level, AssessmentConfidence::Medium);
}

#[test]
fn risk_reduction_never_lowers_deterministic_or_model_risk() {
    let mut checks = passing_checks();
    checks[7].severity = AssessmentRisk::High;

    assert_eq!(
        reduce_risk(&checks, Some(AssessmentRisk::Low)),
        AssessmentRisk::High
    );
    assert_eq!(
        reduce_risk(&passing_checks(), Some(AssessmentRisk::Medium)),
        AssessmentRisk::Medium
    );
}

#[test]
fn routing_lattice_resolves_conflicts_in_documented_order() {
    let cases = [
        route_case(
            "privacy",
            0,
            AssessmentRoute::Drop,
            "privacy_or_invalid_hard_stop",
        ),
        route_case(
            "executable",
            7,
            AssessmentRoute::NeedsHumanReview,
            "safety_or_ambiguity_review",
        ),
        route_case(
            "duplicate",
            2,
            AssessmentRoute::MergeDuplicate,
            "canonical_duplicate",
        ),
        route_case(
            "transient",
            3,
            AssessmentRoute::RecordMemoryOnly,
            "transient_or_pinned",
        ),
        route_case(
            "pinned",
            8,
            AssessmentRoute::RecordMemoryOnly,
            "transient_or_pinned",
        ),
        route_case(
            "insufficient",
            1,
            AssessmentRoute::Drop,
            "ineligible_target_or_guidance",
        ),
    ];

    for (name, mut checks, expected_route, expected_rule) in cases {
        let decision = route_assessment(&RoutingInput {
            classification: SelectionClassification::Selected,
            checks: &checks,
            confidence_bps: 9_000,
            risk: reduce_risk(&checks, None),
            model_route: None,
        });
        assert_eq!(decision.route, expected_route, "case {name}");
        assert_eq!(decision.winning_rule, expected_rule, "case {name}");
        assert_eq!(decision.rules.len(), 8, "case {name}");
        checks.clear();
    }
}

#[test]
fn review_beats_duplicate_and_all_constraints_remain_auditable() {
    let mut checks = passing_checks();
    constrain(
        &mut checks[2],
        AssessmentRoute::MergeDuplicate,
        "canonical_exact_duplicate",
    );
    constrain(
        &mut checks[7],
        AssessmentRoute::NeedsHumanReview,
        "executable_or_side_effect_expansion",
    );

    let decision = route_assessment(&RoutingInput {
        classification: SelectionClassification::Selected,
        checks: &checks,
        confidence_bps: 9_500,
        risk: AssessmentRisk::High,
        model_route: None,
    });

    assert_eq!(decision.route, AssessmentRoute::NeedsHumanReview);
    assert_eq!(decision.route_constraints.len(), 2);
    assert!(decision.rules[0].reason_code.contains("absent"));
    assert_eq!(decision.rules[2].reason_code, "condition_matched");
}

#[test]
fn only_high_confidence_low_risk_all_pass_results_advance() {
    let checks = passing_checks();
    let advanced = route_assessment(&RoutingInput {
        classification: SelectionClassification::Selected,
        checks: &checks,
        confidence_bps: ADVANCE_CONFIDENCE_BPS_V1,
        risk: AssessmentRisk::Low,
        model_route: Some(AssessmentRoute::Advance),
    });
    let low_confidence = route_assessment(&RoutingInput {
        classification: SelectionClassification::Selected,
        checks: &checks,
        confidence_bps: ADVANCE_CONFIDENCE_BPS_V1 - 1,
        risk: AssessmentRisk::Low,
        model_route: None,
    });

    assert_eq!(advanced.route, AssessmentRoute::Advance);
    assert_eq!(advanced.winning_rule, "high_confidence_low_risk");
    assert_eq!(low_confidence.route, AssessmentRoute::NeedsHumanReview);
    assert_eq!(low_confidence.winning_rule, "conservative_default");
}

type RouteCase = (
    &'static str,
    Vec<QualityCheck>,
    AssessmentRoute,
    &'static str,
);

fn route_case(
    name: &'static str,
    check_index: usize,
    route: AssessmentRoute,
    expected_rule: &'static str,
) -> RouteCase {
    let mut checks = passing_checks();
    let reason = match check_index {
        0 => "privacy_residue_detected",
        1 => "insufficient_independent_evidence",
        2 => "canonical_exact_duplicate",
        3 => "transient_or_local_incident",
        7 => "executable_or_side_effect_expansion",
        8 => "target_pinned",
        _ => panic!("unsupported check index"),
    };
    constrain(&mut checks[check_index], route, reason);
    (name, checks, route, expected_rule)
}

fn passing_checks() -> Vec<QualityCheck> {
    QUALITY_CHECK_ORDER_V1
        .iter()
        .map(|kind| QualityCheck {
            kind: *kind,
            result: QualityCheckResult::Pass,
            severity: AssessmentRisk::Low,
            reason_code: "pass".to_string(),
            evidence_ids: Vec::new(),
            route_constraints: Vec::new(),
        })
        .collect()
}

fn constrain(check: &mut QualityCheck, route: AssessmentRoute, reason: &str) {
    check.result = QualityCheckResult::Fail;
    check.reason_code = reason.to_string();
    check.route_constraints = vec![route];
    if route == AssessmentRoute::NeedsHumanReview {
        check.severity = AssessmentRisk::High;
    }
}
