use super::{
    AssessmentConfidence, AssessmentRisk, AssessmentRoute, ConfidenceBreakdown, QualityCheck,
    QualityCheckResult, RoutingDecision, RoutingRuleWitness, SelectionClassification,
};

pub(crate) const ROUTING_POLICY_V1: &str = "routing-v1";
pub(crate) const CONFIDENCE_POLICY_V1: &str = "confidence-v1";
pub(crate) const ADVANCE_CONFIDENCE_BPS_V1: u16 = 8_500;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ModelCorroboration {
    pub(crate) confidence: f32,
    pub(crate) citations_valid: bool,
}

pub(crate) struct ConfidenceInput<'a> {
    pub(crate) verified_corrected_feedback: bool,
    pub(crate) independent_nonduplicate_runs: u8,
    pub(crate) selection_score: u8,
    pub(crate) selection_margin: u8,
    pub(crate) independent_lineages: u8,
    pub(crate) checks: &'a [QualityCheck],
    pub(crate) material_contradiction: bool,
    pub(crate) model: Option<ModelCorroboration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfidenceAssessment {
    pub(crate) level: AssessmentConfidence,
    pub(crate) breakdown: ConfidenceBreakdown,
}

pub(crate) struct RoutingInput<'a> {
    pub(crate) classification: SelectionClassification,
    pub(crate) checks: &'a [QualityCheck],
    pub(crate) confidence_bps: u16,
    pub(crate) risk: AssessmentRisk,
    pub(crate) model_route: Option<AssessmentRoute>,
}

pub(crate) fn calculate_confidence(input: &ConfidenceInput<'_>) -> ConfidenceAssessment {
    let evidence_strength_bps = if input.verified_corrected_feedback {
        3_000
    } else if input.independent_nonduplicate_runs >= 2 {
        2_500
    } else {
        1_000
    };
    let selection_score_bps = u16::from(input.selection_score.min(100)) * 25;
    let selection_margin_bps = u16::from(input.selection_margin.min(15)) * 100;
    let lineage_independence_bps = u16::from(input.independent_lineages.min(2)) * 750;
    let evaluated_checks = input
        .checks
        .iter()
        .filter(|check| check.result != QualityCheckResult::NotApplicable)
        .count()
        .min(9) as u16;
    let check_completeness_bps = evaluated_checks * 1_000 / 9;
    let contradiction_penalty_bps = if input.material_contradiction {
        2_000
    } else {
        0
    };
    let model_corroboration_bps = input
        .model
        .filter(|model| model.citations_valid && model.confidence.is_finite())
        .map(|model| (model.confidence.clamp(0.0, 1.0) * 500.0).round() as u16)
        .unwrap_or(0);
    let positive = evidence_strength_bps
        + selection_score_bps
        + selection_margin_bps
        + lineage_independence_bps
        + check_completeness_bps
        + model_corroboration_bps;
    let system_confidence_bps = positive
        .saturating_sub(contradiction_penalty_bps)
        .min(10_000);
    let level = if system_confidence_bps >= ADVANCE_CONFIDENCE_BPS_V1 {
        AssessmentConfidence::High
    } else if system_confidence_bps >= 6_000 {
        AssessmentConfidence::Medium
    } else {
        AssessmentConfidence::Low
    };
    ConfidenceAssessment {
        level,
        breakdown: ConfidenceBreakdown {
            policy_version: CONFIDENCE_POLICY_V1.to_string(),
            evidence_strength_bps,
            selection_score_bps,
            selection_margin_bps,
            lineage_independence_bps,
            check_completeness_bps,
            contradiction_penalty_bps,
            model_corroboration_bps,
            system_confidence_bps,
        },
    }
}

pub(crate) fn reduce_risk(
    checks: &[QualityCheck],
    valid_model_risk: Option<AssessmentRisk>,
) -> AssessmentRisk {
    checks
        .iter()
        .map(|check| check.severity)
        .chain(valid_model_risk)
        .max_by_key(|risk| risk_rank(*risk))
        .unwrap_or(AssessmentRisk::Low)
}

pub(crate) fn route_assessment(input: &RoutingInput<'_>) -> RoutingDecision {
    let constraints = collect_constraints(input);
    let rule_matches = [
        matches_reason(input.checks, &["privacy_residue_detected"]),
        matches!(input.model_route, Some(AssessmentRoute::Drop)),
        has_constraint(&constraints, AssessmentRoute::NeedsHumanReview)
            || input.classification == SelectionClassification::Ambiguous,
        has_constraint(&constraints, AssessmentRoute::MergeDuplicate),
        has_constraint(&constraints, AssessmentRoute::RecordMemoryOnly),
        has_constraint(&constraints, AssessmentRoute::Drop)
            || input.classification == SelectionClassification::NoTarget,
        all_checks_pass(input.checks)
            && input.classification == SelectionClassification::Selected
            && input.risk == AssessmentRisk::Low
            && input.confidence_bps >= ADVANCE_CONFIDENCE_BPS_V1,
        true,
    ];
    let definitions = [
        ("privacy_or_invalid_hard_stop", AssessmentRoute::Drop),
        ("model_stricter_drop", AssessmentRoute::Drop),
        (
            "safety_or_ambiguity_review",
            AssessmentRoute::NeedsHumanReview,
        ),
        ("canonical_duplicate", AssessmentRoute::MergeDuplicate),
        ("transient_or_pinned", AssessmentRoute::RecordMemoryOnly),
        ("ineligible_target_or_guidance", AssessmentRoute::Drop),
        ("high_confidence_low_risk", AssessmentRoute::Advance),
        ("conservative_default", AssessmentRoute::NeedsHumanReview),
    ];
    let winning_index = rule_matches
        .iter()
        .position(|matched| *matched)
        .unwrap_or(7);
    let rules = definitions
        .iter()
        .zip(rule_matches)
        .map(|((code, route), matched)| RoutingRuleWitness {
            rule_code: (*code).to_string(),
            route: *route,
            matched,
            reason_code: if matched {
                "condition_matched"
            } else {
                "stricter_condition_absent"
            }
            .to_string(),
        })
        .collect();
    RoutingDecision {
        policy_version: ROUTING_POLICY_V1.to_string(),
        route: definitions[winning_index].1,
        winning_rule: definitions[winning_index].0.to_string(),
        route_constraints: constraints,
        rules,
    }
}

fn collect_constraints(input: &RoutingInput<'_>) -> Vec<AssessmentRoute> {
    let mut constraints = input
        .checks
        .iter()
        .flat_map(|check| check.route_constraints.iter().copied())
        .collect::<Vec<_>>();
    if let Some(route) = input.model_route {
        constraints.push(route);
    }
    constraints.sort_by_key(|route| route_rank(*route));
    constraints.dedup();
    constraints
}

fn matches_reason(checks: &[QualityCheck], reasons: &[&str]) -> bool {
    checks
        .iter()
        .any(|check| reasons.contains(&check.reason_code.as_str()))
}

fn has_constraint(constraints: &[AssessmentRoute], route: AssessmentRoute) -> bool {
    constraints.contains(&route)
}

fn all_checks_pass(checks: &[QualityCheck]) -> bool {
    checks.len() == 9
        && checks
            .iter()
            .all(|check| check.result == QualityCheckResult::Pass)
}

fn route_rank(route: AssessmentRoute) -> u8 {
    match route {
        AssessmentRoute::Advance => 0,
        AssessmentRoute::RecordMemoryOnly => 1,
        AssessmentRoute::MergeDuplicate => 2,
        AssessmentRoute::NeedsHumanReview => 3,
        AssessmentRoute::Drop => 4,
    }
}

fn risk_rank(risk: AssessmentRisk) -> u8 {
    match risk {
        AssessmentRisk::Low => 0,
        AssessmentRisk::Medium => 1,
        AssessmentRisk::High => 2,
    }
}
