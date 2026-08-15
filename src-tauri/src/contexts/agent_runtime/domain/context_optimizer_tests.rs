use super::*;

fn component(
    sequence: usize,
    round: Option<usize>,
    retention_class: RetentionClass,
    characters: u64,
    tokens: Option<u64>,
) -> ContextComponent {
    ContextComponent {
        sequence,
        semantic_class: SemanticClass::AssistantResponse,
        retention_class,
        round,
        characters,
        estimated_tokens: tokens,
        content_fingerprint: format!("fingerprint-{sequence}"),
        tool_reference: None,
        current_user_intent: false,
        correction: false,
        reinjectable: retention_class == RetentionClass::Reinjectable,
        repeated_tool_result: retention_class == RetentionClass::Microcompactable,
    }
}

fn snapshot(components: Vec<ContextComponent>, rounds: Vec<ContextRound>) -> ContextSnapshot {
    let characters = components
        .iter()
        .fold(0_u64, |total, item| total.saturating_add(item.characters));
    let tokens = components.iter().try_fold(0_u64, |total, item| {
        item.estimated_tokens
            .map(|value| total.saturating_add(value))
    });
    ContextSnapshot {
        version: CONTEXT_SNAPSHOT_VERSION,
        estimator_version: CONTEXT_ESTIMATOR_VERSION,
        policy_version: CONTEXT_POLICY_VERSION,
        request_fingerprint: "request-fingerprint".to_string(),
        quality: MeasurementQuality::Estimated,
        characters,
        tokens,
        components,
        rounds,
        capacity: None,
        reserved_tokens: None,
        remaining_tokens: None,
        utilization_basis_points: None,
        active_character_compaction: true,
        compaction_decision: ContextCompactionDecision::evaluate(None, None),
        overflow_count: 0,
    }
}

fn round(index: usize, state: ProtocolState, sequences: Vec<usize>) -> ContextRound {
    ContextRound {
        index,
        protocol_state: state,
        component_sequences: sequences,
    }
}

fn budget(
    snapshot: &ContextSnapshot,
    target_characters: u64,
    target_tokens: Option<u64>,
) -> ContextOptimizationBudget {
    ContextOptimizationBudget {
        original_characters: snapshot.characters,
        original_tokens: snapshot.tokens,
        target_characters,
        target_tokens,
    }
}

#[test]
fn planner_orders_low_cost_actions_and_stops_at_exact_target() {
    let value = snapshot(
        vec![
            component(
                0,
                Some(0),
                RetentionClass::Microcompactable,
                1_000,
                Some(250),
            ),
            component(1, Some(1), RetentionClass::Reinjectable, 400, Some(100)),
            component(2, None, RetentionClass::Discardable, 200, Some(50)),
            component(3, Some(2), RetentionClass::Verbatim, 400, Some(100)),
        ],
        vec![
            round(0, ProtocolState::Complete, vec![0]),
            round(1, ProtocolState::Complete, vec![1]),
            round(2, ProtocolState::Complete, vec![3]),
        ],
    );
    let plan = build_optimization_plan(&value, budget(&value, 1_400, Some(350))).expect("plan");
    assert_eq!(
        plan.actions
            .iter()
            .map(|action| action.kind)
            .collect::<Vec<_>>(),
        vec![
            OptimizationActionKind::DiscardTransient,
            OptimizationActionKind::ReplaceReinjectable,
        ]
    );
    assert_eq!(plan.projected_tokens, Some(350));
    assert_eq!(plan.outcome, OptimizationOutcome::Planned);
}

#[test]
fn planner_preserves_protected_unknown_verbatim_and_incomplete_protocol() {
    let value = snapshot(
        vec![
            component(0, Some(0), RetentionClass::Discardable, 500, Some(125)),
            component(1, Some(1), RetentionClass::Protected, 500, Some(125)),
            component(2, Some(2), RetentionClass::Verbatim, 500, Some(125)),
        ],
        vec![
            round(0, ProtocolState::Incomplete, vec![0]),
            round(1, ProtocolState::Complete, vec![1]),
            round(2, ProtocolState::Complete, vec![2]),
        ],
    );
    let plan = build_optimization_plan(&value, budget(&value, 0, Some(0))).expect("plan");
    assert!(plan.actions.is_empty());
    assert_eq!(
        plan.outcome,
        OptimizationOutcome::InsufficientReclaimableContext
    );
}

#[test]
fn planner_selects_only_contiguous_oldest_complete_summarizable_rounds() {
    let value = snapshot(
        vec![
            component(0, Some(0), RetentionClass::Summarizable, 400, Some(100)),
            component(1, Some(1), RetentionClass::Summarizable, 400, Some(100)),
            component(2, Some(2), RetentionClass::Verbatim, 400, Some(100)),
        ],
        vec![
            round(0, ProtocolState::Complete, vec![0]),
            round(1, ProtocolState::Complete, vec![1]),
            round(2, ProtocolState::Complete, vec![2]),
        ],
    );
    let plan = build_optimization_plan(&value, budget(&value, 400, Some(100))).expect("plan");
    assert_eq!(
        plan.summary_boundary,
        Some(SummaryBoundary {
            first_round: 0,
            last_round: 1,
            round_count: 2,
        })
    );
    assert_eq!(plan.actions.len(), 2);
    assert_eq!(plan.projected_tokens, Some(100));
}

#[test]
fn planner_handles_empty_character_only_saturation_and_invalid_budget() {
    let empty = snapshot(Vec::new(), Vec::new());
    let empty_plan = build_optimization_plan(&empty, budget(&empty, 0, None)).expect("empty");
    assert_eq!(empty_plan.outcome, OptimizationOutcome::TargetSatisfied);

    let character_only = snapshot(
        vec![component(
            0,
            None,
            RetentionClass::Discardable,
            u64::MAX,
            None,
        )],
        Vec::new(),
    );
    let plan = build_optimization_plan(&character_only, budget(&character_only, 0, None))
        .expect("character plan");
    assert_eq!(plan.projected_characters, 0);
    assert_eq!(plan.projected_tokens, None);

    let invalid = ContextOptimizationBudget {
        original_characters: u64::MAX - 1,
        ..budget(&character_only, 0, None)
    };
    assert_eq!(
        build_optimization_plan(&character_only, invalid),
        Err(OptimizationPlanError::InvalidBudget)
    );
}

#[test]
fn every_action_references_known_non_overlapping_safe_content() {
    let value = snapshot(
        vec![
            component(0, Some(0), RetentionClass::Summarizable, 300, Some(75)),
            component(1, Some(1), RetentionClass::Microcompactable, 800, Some(200)),
            component(2, Some(2), RetentionClass::Protected, 300, Some(75)),
        ],
        vec![
            round(0, ProtocolState::Complete, vec![0]),
            round(1, ProtocolState::Complete, vec![1]),
            round(2, ProtocolState::Complete, vec![2]),
        ],
    );
    let plan = build_optimization_plan(&value, budget(&value, 500, Some(125))).expect("plan");
    let mut targets = std::collections::HashSet::new();
    for action in &plan.actions {
        assert!(targets.insert(action.target.clone()));
        assert!(action
            .source_fingerprints
            .iter()
            .all(|fingerprint| fingerprint.as_str().starts_with("fingerprint-")));
        assert_ne!(action.target, OptimizationTarget::Component(2));
        assert_ne!(action.target, OptimizationTarget::Round(2));
    }
}

#[test]
fn unsafe_fingerprints_and_unknown_round_members_are_rejected() {
    let mut unsafe_value = snapshot(
        vec![component(0, None, RetentionClass::Discardable, 10, Some(3))],
        Vec::new(),
    );
    unsafe_value.components[0].content_fingerprint = "raw content!".to_string();
    assert_eq!(
        build_optimization_plan(&unsafe_value, budget(&unsafe_value, 0, Some(0))),
        Err(OptimizationPlanError::UnsafeFingerprint)
    );

    let invalid = snapshot(
        vec![component(
            0,
            Some(0),
            RetentionClass::Summarizable,
            10,
            Some(3),
        )],
        vec![round(0, ProtocolState::Complete, vec![99])],
    );
    assert_eq!(
        build_optimization_plan(&invalid, budget(&invalid, 0, Some(0))),
        Err(OptimizationPlanError::InvalidSnapshot)
    );
}

fn verification_fixture() -> (ContextSnapshot, ContextSnapshot, ContextOptimizationPlan) {
    let original = snapshot(
        vec![
            component(0, Some(0), RetentionClass::Summarizable, 400, Some(100)),
            component(1, Some(1), RetentionClass::Protected, 300, Some(75)),
            component(2, Some(2), RetentionClass::Verbatim, 300, Some(75)),
        ],
        vec![
            round(0, ProtocolState::Complete, vec![0]),
            round(1, ProtocolState::Complete, vec![1]),
            round(2, ProtocolState::Complete, vec![2]),
        ],
    );
    let plan = build_optimization_plan(&original, budget(&original, 600, Some(150)))
        .expect("verification plan");
    let candidate = snapshot(
        vec![
            component(1, Some(0), RetentionClass::Protected, 300, Some(75)),
            component(2, Some(1), RetentionClass::Verbatim, 300, Some(75)),
        ],
        vec![
            round(0, ProtocolState::Complete, vec![1]),
            round(1, ProtocolState::Complete, vec![2]),
        ],
    );
    (original, candidate, plan)
}

#[test]
fn verifier_accepts_reducing_candidate_and_uses_character_fallback() {
    let (original, candidate, plan) = verification_fixture();
    let verification = verify_optimization_candidate(&original, &candidate, &plan, &[]);
    assert!(verification.accepted);
    assert!(verification.failures.is_empty());
    assert_eq!(
        verification.candidate.expect("evidence").reduction_basis,
        ReductionBasis::Tokens
    );

    let mut character_original = original;
    let mut character_candidate = candidate;
    character_original.tokens = None;
    character_candidate.tokens = None;
    let character_plan = ContextOptimizationPlan {
        budget: ContextOptimizationBudget {
            original_tokens: None,
            target_tokens: None,
            ..plan.budget
        },
        projected_tokens: None,
        ..plan
    };
    let verification = verify_optimization_candidate(
        &character_original,
        &character_candidate,
        &character_plan,
        &[],
    );
    assert!(verification.accepted);
    assert_eq!(
        verification.candidate.expect("evidence").reduction_basis,
        ReductionBasis::Characters
    );
}

#[test]
fn verifier_rejects_missing_reordered_or_changed_protected_context() {
    let (original, candidate, plan) = verification_fixture();
    let mut missing = candidate.clone();
    missing.components.remove(1);
    let result = verify_optimization_candidate(&original, &missing, &plan, &[]);
    assert!(result
        .failures
        .contains(&VerificationFailure::VerbatimContentChanged));

    let mut reordered = candidate.clone();
    reordered.components.swap(0, 1);
    let result = verify_optimization_candidate(&original, &reordered, &plan, &[]);
    assert!(result
        .failures
        .contains(&VerificationFailure::ComponentOrderChanged));

    let mut changed_intent = candidate;
    changed_intent.components[1].content_fingerprint = "changed-intent".to_string();
    let result = verify_optimization_candidate(&original, &changed_intent, &plan, &[]);
    assert!(result
        .failures
        .contains(&VerificationFailure::VerbatimContentChanged));
}

#[test]
fn verifier_rejects_protocol_action_reinjection_and_coverage_mutations() {
    let (original, candidate, plan) = verification_fixture();
    let mut incomplete = candidate.clone();
    incomplete.rounds[0].protocol_state = ProtocolState::Incomplete;
    let result = verify_optimization_candidate(&original, &incomplete, &plan, &[]);
    assert!(result
        .failures
        .contains(&VerificationFailure::ProtocolIncomplete));

    let mut action_mismatch = plan.clone();
    action_mismatch.actions[0].source_fingerprints = vec![SafeFingerprint::parse("wrong").unwrap()];
    let result = verify_optimization_candidate(&original, &candidate, &action_mismatch, &[]);
    assert!(result
        .failures
        .contains(&VerificationFailure::ActionMismatch));

    let required = [SafeFingerprint::parse("current-memory").unwrap()];
    let result = verify_optimization_candidate(&original, &candidate, &plan, &required);
    assert!(result
        .failures
        .contains(&VerificationFailure::ReinjectionMissing));

    let mut uncovered = candidate;
    uncovered.characters = uncovered.characters.saturating_add(1);
    uncovered.overflow_count = 1;
    let result = verify_optimization_candidate(&original, &uncovered, &plan, &[]);
    assert!(result
        .failures
        .contains(&VerificationFailure::CoverageIncomplete));
}

#[test]
fn verifier_rejects_equal_larger_and_target_violating_candidates() {
    let (original, candidate, plan) = verification_fixture();
    let mut equal = candidate.clone();
    equal.characters = original.characters;
    equal.tokens = original.tokens;
    let result = verify_optimization_candidate(&original, &equal, &plan, &[]);
    assert!(result
        .failures
        .contains(&VerificationFailure::CandidateNotSmaller));

    let mut target_violating = candidate;
    target_violating.characters = 700;
    target_violating.tokens = Some(175);
    let result = verify_optimization_candidate(&original, &target_violating, &plan, &[]);
    assert!(result.failures.contains(&VerificationFailure::TargetNotMet));
    assert!(!result.accepted);
    assert_eq!(
        result.fallback_reason,
        Some(FallbackReason::VerificationFailed)
    );
}
