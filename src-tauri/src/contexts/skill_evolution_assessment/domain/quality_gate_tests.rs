use super::*;

#[test]
fn quality_registry_always_returns_exactly_nine_ordered_results() {
    let assessment = evaluate_quality_gates(&passing_input());

    assert_eq!(assessment.checks.len(), 9);
    assert_eq!(
        assessment
            .checks
            .iter()
            .map(|check| check.kind)
            .collect::<Vec<_>>(),
        QUALITY_CHECK_ORDER_V1
    );
    assert!(assessment
        .checks
        .iter()
        .all(|check| check.result == QualityCheckResult::Pass));
    assert!(assessment.model_evaluation_allowed);
}

#[test]
fn privacy_residue_hard_stops_models_without_shortening_the_audit() {
    let mut input = passing_input();
    input.privacy_residue = true;
    let assessment = evaluate_quality_gates(&input);

    assert!(!assessment.model_evaluation_allowed);
    assert_eq!(assessment.checks.len(), 9);
    assert_eq!(assessment.checks[0].result, QualityCheckResult::Fail);
    assert_eq!(
        assessment.checks[0].route_constraints,
        vec![AssessmentRoute::Drop]
    );
    assert!(assessment.checks[1..]
        .iter()
        .all(|check| check.result == QualityCheckResult::NotApplicable));
}

#[test]
fn quality_corpus_covers_constraints_reviews_and_lifecycle_states() {
    let cases = [
        case_insufficient(),
        case_duplicate(),
        case_transient(),
        case_vague(),
        case_contradiction(),
        case_uncertain_target(),
        case_incompatible_target(),
        case_executable(),
        case_pinned(),
        case_archived(),
        case_missing(),
        case_malformed(),
        case_changed_revision(),
    ];
    for (name, input, kind, result, route) in cases {
        let assessment = evaluate_quality_gates(&input);
        let check = assessment
            .checks
            .iter()
            .find(|check| check.kind == kind)
            .unwrap_or_else(|| panic!("missing check for {name}"));
        assert_eq!(check.result, result, "case {name}");
        assert!(check.route_constraints.contains(&route), "case {name}");
    }
}

#[test]
fn scoped_contradictions_remain_consistent() {
    let mut input = passing_input();
    input.material_contradiction = true;
    input.contradiction_is_scoped = true;

    let assessment = evaluate_quality_gates(&input);
    let check = &assessment.checks[5];
    assert_eq!(check.result, QualityCheckResult::Pass);
    assert_eq!(check.reason_code, "scoped_evidence_consistent");
}

#[test]
fn content_kind_alone_detects_executable_risk() {
    let mut input = passing_input();
    input.lesson_shape.content_kinds = vec!["tool_declaration".to_string()];

    let assessment = evaluate_quality_gates(&input);
    assert_eq!(assessment.checks[7].result, QualityCheckResult::Review);
    assert_eq!(assessment.checks[7].severity, AssessmentRisk::High);
}

#[test]
fn independent_quality_constraints_are_all_preserved() {
    let mut input = passing_input();
    input.duplicate = DuplicateAssessment {
        classification: Some(DuplicateClassification::Exact),
        canonical_reference: Some("skill:r1".to_string()),
        risk_references: Vec::new(),
    };
    input.executable_content = true;
    input.target = Some(target(TargetLifecycle::Pinned, false));

    let assessment = evaluate_quality_gates(&input);
    let constraints = assessment
        .checks
        .iter()
        .flat_map(|check| check.route_constraints.iter().copied())
        .collect::<Vec<_>>();
    assert!(constraints.contains(&AssessmentRoute::MergeDuplicate));
    assert!(constraints.contains(&AssessmentRoute::NeedsHumanReview));
    assert!(constraints.contains(&AssessmentRoute::RecordMemoryOnly));
}

fn passing_input() -> QualityGateInput {
    QualityGateInput {
        privacy_residue: false,
        verified_corrected_feedback: true,
        independent_nonduplicate_runs: 1,
        duplicate: DuplicateAssessment {
            classification: None,
            canonical_reference: None,
            risk_references: Vec::new(),
        },
        transient_incident: false,
        lesson_shape: LessonShape {
            trigger: Some("verification_failure".to_string()),
            required_behavior: Some("inspect_evidence".to_string()),
            prohibited_behavior: None,
            verification: Some("test_passes".to_string()),
            environment: Some("project".to_string()),
            content_kinds: vec!["guidance".to_string()],
        },
        material_contradiction: false,
        contradiction_is_scoped: false,
        target: Some(target(TargetLifecycle::Active, false)),
        target_compatible: true,
        target_revision_current: true,
        executable_content: false,
        evidence_ids: vec!["evidence-b".to_string(), "evidence-a".to_string()],
    }
}

fn target(lifecycle: TargetLifecycle, uncertain: bool) -> RankedTarget {
    RankedTarget {
        witness: EffectiveTargetWitness {
            skill_id: "review".to_string(),
            skill_type: "role".to_string(),
            revision_hash: "r1".to_string(),
            scope: TargetScope::Project,
            lifecycle,
            trust: TargetTrust::Trusted,
        },
        score: 80,
        attribution_score: if uncertain { 20 } else { 35 },
        participation_score: 15,
        compatibility_score: 15,
        lexical_score: 10,
        locality_score: 5,
        matched_feature_classes: vec!["capability".to_string()],
        exclusions: Vec::new(),
        attribution_uncertain: uncertain,
    }
}

type QualityCase = (
    &'static str,
    QualityGateInput,
    QualityCheckKind,
    QualityCheckResult,
    AssessmentRoute,
);

fn case_insufficient() -> QualityCase {
    let mut input = passing_input();
    input.verified_corrected_feedback = false;
    input.independent_nonduplicate_runs = 1;
    (
        "insufficient",
        input,
        QualityCheckKind::EvidenceSufficiency,
        QualityCheckResult::Fail,
        AssessmentRoute::Drop,
    )
}

fn case_duplicate() -> QualityCase {
    let mut input = passing_input();
    input.duplicate = DuplicateAssessment {
        classification: Some(DuplicateClassification::Exact),
        canonical_reference: Some("skill:r1".to_string()),
        risk_references: Vec::new(),
    };
    (
        "duplicate",
        input,
        QualityCheckKind::DuplicateKnowledge,
        QualityCheckResult::Fail,
        AssessmentRoute::MergeDuplicate,
    )
}

fn case_transient() -> QualityCase {
    let mut input = passing_input();
    input.transient_incident = true;
    (
        "transient",
        input,
        QualityCheckKind::TransientIncident,
        QualityCheckResult::Fail,
        AssessmentRoute::RecordMemoryOnly,
    )
}

fn case_vague() -> QualityCase {
    let mut input = passing_input();
    input.lesson_shape.trigger = None;
    (
        "vague",
        input,
        QualityCheckKind::GuidanceSpecificity,
        QualityCheckResult::Fail,
        AssessmentRoute::Drop,
    )
}

fn case_contradiction() -> QualityCase {
    let mut input = passing_input();
    input.material_contradiction = true;
    (
        "contradiction",
        input,
        QualityCheckKind::EvidenceConsistency,
        QualityCheckResult::Review,
        AssessmentRoute::NeedsHumanReview,
    )
}

fn case_uncertain_target() -> QualityCase {
    let mut input = passing_input();
    input.target = Some(target(TargetLifecycle::Active, true));
    (
        "uncertain-target",
        input,
        QualityCheckKind::TargetCompatibility,
        QualityCheckResult::Review,
        AssessmentRoute::NeedsHumanReview,
    )
}

fn case_incompatible_target() -> QualityCase {
    let mut input = passing_input();
    input.target_compatible = false;
    (
        "incompatible-target",
        input,
        QualityCheckKind::TargetCompatibility,
        QualityCheckResult::Fail,
        AssessmentRoute::Drop,
    )
}

fn case_executable() -> QualityCase {
    let mut input = passing_input();
    input.executable_content = true;
    (
        "executable",
        input,
        QualityCheckKind::ExecutableContentRisk,
        QualityCheckResult::Review,
        AssessmentRoute::NeedsHumanReview,
    )
}

fn case_pinned() -> QualityCase {
    let mut input = passing_input();
    input.target = Some(target(TargetLifecycle::Pinned, false));
    (
        "pinned",
        input,
        QualityCheckKind::TargetLifecycleMutability,
        QualityCheckResult::Fail,
        AssessmentRoute::RecordMemoryOnly,
    )
}

fn case_archived() -> QualityCase {
    let mut input = passing_input();
    input.target = Some(target(TargetLifecycle::Archived, false));
    (
        "archived",
        input,
        QualityCheckKind::TargetLifecycleMutability,
        QualityCheckResult::Fail,
        AssessmentRoute::Drop,
    )
}

fn case_missing() -> QualityCase {
    let mut input = passing_input();
    input.target = Some(target(TargetLifecycle::Missing, false));
    (
        "missing",
        input,
        QualityCheckKind::TargetLifecycleMutability,
        QualityCheckResult::Fail,
        AssessmentRoute::Drop,
    )
}

fn case_malformed() -> QualityCase {
    let mut input = passing_input();
    input.target = Some(target(TargetLifecycle::Malformed, false));
    (
        "malformed",
        input,
        QualityCheckKind::TargetLifecycleMutability,
        QualityCheckResult::Fail,
        AssessmentRoute::Drop,
    )
}

fn case_changed_revision() -> QualityCase {
    let mut input = passing_input();
    input.target_revision_current = false;
    (
        "changed-revision",
        input,
        QualityCheckKind::TargetLifecycleMutability,
        QualityCheckResult::Fail,
        AssessmentRoute::Drop,
    )
}
