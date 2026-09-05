use super::*;

#[test]
fn lesson_shape_is_derived_only_from_bounded_structured_evidence() {
    let shape = derive_lesson_shape(&StructuredLessonEvidence {
        trigger: Some(LessonTrigger::VerificationFailure),
        required_behavior: Some(LessonBehavior::InspectEvidence),
        prohibited_behavior: None,
        verification: Some(LessonVerification::TestPasses),
        environment: Some(LessonEnvironment::Project),
        content_kinds: vec![
            LessonContentKind::Reference,
            LessonContentKind::Guidance,
            LessonContentKind::Reference,
        ],
    });

    assert_eq!(shape.trigger.as_deref(), Some("verification_failure"));
    assert_eq!(shape.required_behavior.as_deref(), Some("inspect_evidence"));
    assert_eq!(shape.prohibited_behavior, None);
    assert_eq!(shape.verification.as_deref(), Some("test_passes"));
    assert_eq!(shape.environment.as_deref(), Some("project"));
    assert_eq!(shape.content_kinds, vec!["guidance", "reference"]);
}

#[test]
fn exact_and_conservative_near_duplicates_require_structural_compatibility() {
    let candidate = unit(
        GuidanceSourceKind::PendingAssessment,
        "candidate",
        shape(
            "When verification fails",
            "Inspect failure evidence",
            "before retry",
            "tests pass",
            "project",
        ),
    );
    let exact = unit(
        GuidanceSourceKind::EffectiveSkill,
        "skill:exact",
        shape(
            "  WHEN verification fails ",
            "INSPECT failure evidence",
            "before retry",
            "tests pass",
            "project",
        ),
    );
    let near = unit(
        GuidanceSourceKind::TrustedOverlay,
        "overlay:near",
        shape(
            "when verification fails",
            "inspect failure evidence carefully",
            "before retry",
            "tests pass",
            "project",
        ),
    );

    let exact_result = assess_duplicate(&candidate, &[exact]);
    assert_eq!(
        exact_result.classification,
        Some(DuplicateClassification::Exact)
    );
    assert_eq!(
        exact_result.canonical_reference.as_deref(),
        Some("skill:exact")
    );
    let near_result = assess_duplicate(&candidate, &[near]);
    assert_eq!(
        near_result.classification,
        Some(DuplicateClassification::Near)
    );
}

#[test]
fn shared_terms_and_scoped_variants_do_not_collapse_distinct_guidance() {
    let candidate = unit(
        GuidanceSourceKind::PendingAssessment,
        "candidate",
        shape(
            "when verification fails",
            "inspect failure evidence",
            "before retry",
            "tests pass",
            "project",
        ),
    );
    let different_behavior = unit(
        GuidanceSourceKind::EffectiveSkill,
        "skill:different",
        shape(
            "when verification fails",
            "ignore failure evidence",
            "before retry",
            "tests pass",
            "project",
        ),
    );
    let different_scope = unit(
        GuidanceSourceKind::EffectiveSkill,
        "skill:user",
        shape(
            "when verification fails",
            "inspect failure evidence",
            "before retry",
            "tests pass",
            "user",
        ),
    );

    assert_eq!(
        assess_duplicate(&candidate, &[different_behavior, different_scope]).classification,
        None
    );
}

#[test]
fn pending_guidance_is_canonical_but_untrusted_overlay_is_only_risk_evidence() {
    let candidate = unit(
        GuidanceSourceKind::EffectiveSkill,
        "candidate",
        shape("trigger", "action", "constraint", "verify", "project"),
    );
    let pending = unit(
        GuidanceSourceKind::PendingAssessment,
        "pending:1",
        shape("trigger", "action", "constraint", "verify", "project"),
    );
    let untrusted = unit(
        GuidanceSourceKind::UntrustedOverlay,
        "overlay:untrusted",
        shape("trigger", "action", "constraint", "verify", "project"),
    );

    let pending_result = assess_duplicate(&candidate, &[pending]);
    assert_eq!(
        pending_result.canonical_reference.as_deref(),
        Some("pending:1")
    );
    let untrusted_result = assess_duplicate(&candidate, &[untrusted]);
    assert_eq!(untrusted_result.classification, None);
    assert_eq!(untrusted_result.risk_references, vec!["overlay:untrusted"]);
}

fn unit(source: GuidanceSourceKind, reference: &str, shape: LessonShape) -> GuidanceUnit {
    build_guidance_units(&[GuidanceUnitInput {
        source,
        reference: reference.to_string(),
        shape,
    }])
    .remove(0)
}

fn shape(
    trigger: &str,
    action: &str,
    constraint: &str,
    verification: &str,
    environment: &str,
) -> LessonShape {
    LessonShape {
        trigger: Some(trigger.to_string()),
        required_behavior: Some(action.to_string()),
        prohibited_behavior: Some(constraint.to_string()),
        verification: Some(verification.to_string()),
        environment: Some(environment.to_string()),
        content_kinds: vec!["guidance".to_string()],
    }
}
