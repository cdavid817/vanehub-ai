use super::*;

fn input(outcome: ContextAssessmentOutcome) -> ContextQualityAssessmentInput<'static> {
    ContextQualityAssessmentInput {
        generation_correlation: "generation-private-prompt",
        decision_sequence: 3,
        outcome,
        path: Some(ContextAssessmentPath::Optimizer),
        reason: None,
        trigger_source: Some(ContextAssessmentTriggerSource::TokenAware),
        before_characters: 10_000,
        after_characters: 4_000,
        before_tokens: Some(2_500),
        after_tokens: Some(1_000),
        measurement_quality: ContextAssessmentMeasurementQuality::Reported,
        invariants: Some(ContextAssessmentInvariants::passed()),
        context_policy_version: "context-policy-v1",
        optimizer_version: "optimizer-v1",
        verifier_version: "verifier-v1",
    }
}

#[test]
fn assessment_outcomes_and_bounded_reasons_cover_final_decisions() {
    for outcome in [
        ContextAssessmentOutcome::Compacted,
        ContextAssessmentOutcome::Bypassed,
        ContextAssessmentOutcome::Fallback,
        ContextAssessmentOutcome::Failed,
    ] {
        assert_eq!(
            ContextQualityAssessment::new(input(outcome)).outcome,
            outcome
        );
    }

    assert_eq!(
        ContextAssessmentReason::from(CompactionBypassReason::UserPreferenceSuppressed),
        ContextAssessmentReason::UserPreferenceSuppressed
    );
    assert_eq!(
        ContextAssessmentReason::from(FallbackReason::VerificationFailed),
        ContextAssessmentReason::VerificationFailed
    );
}

#[test]
fn savings_are_saturating_and_tokens_remain_explicitly_optional() {
    let mut values = input(ContextAssessmentOutcome::Compacted);
    values.after_characters = u64::MAX;
    values.after_tokens = Some(u64::MAX);
    let assessment = ContextQualityAssessment::new(values);

    assert_eq!(assessment.saved_characters, 0);
    assert_eq!(assessment.saved_tokens, Some(0));

    let mut character_only = input(ContextAssessmentOutcome::Fallback);
    character_only.before_tokens = None;
    character_only.after_tokens = None;
    character_only.measurement_quality = ContextAssessmentMeasurementQuality::CharactersOnly;
    assert_eq!(
        ContextQualityAssessment::new(character_only).saved_tokens,
        None
    );
}

#[test]
fn attempt_correlation_is_stable_bounded_and_sequence_specific() {
    let first = ContextQualityAssessment::new(input(ContextAssessmentOutcome::Compacted));
    let repeated = ContextQualityAssessment::new(input(ContextAssessmentOutcome::Compacted));
    let mut next_input = input(ContextAssessmentOutcome::Compacted);
    next_input.decision_sequence += 1;
    let next = ContextQualityAssessment::new(next_input);

    assert_eq!(first.attempt_id, repeated.attempt_id);
    assert_ne!(first.attempt_id, next.attempt_id);
    assert!(first.attempt_id.len() < 40);
    assert!(first.attempt_id.starts_with("ctxq-"));
}

#[test]
fn serialized_assessment_has_no_source_content_fields_or_values() {
    let assessment = ContextQualityAssessment::new(input(ContextAssessmentOutcome::Compacted));
    let serialized = serde_json::to_string(&assessment).expect("assessment serialization");

    assert!(assessment
        .invariants
        .is_some_and(ContextAssessmentInvariants::all_passed));
    for forbidden in [
        "private-prompt",
        "prompt",
        "summary",
        "toolResult",
        "toolArguments",
        "authorization",
        "secret",
    ] {
        assert!(!serialized
            .to_ascii_lowercase()
            .contains(&forbidden.to_ascii_lowercase()));
    }
}
