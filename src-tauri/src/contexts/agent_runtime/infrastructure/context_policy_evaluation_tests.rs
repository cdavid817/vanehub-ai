use super::*;
use crate::contexts::agent_runtime::infrastructure::context_policy_corpus::{
    context_policy_regression_corpus, CorpusAdjustment, CONTEXT_POLICY_CORPUS_VERSION,
};
use crate::contexts::agent_runtime::infrastructure::context_policy_evaluation_support::adjusted_snapshot;
use crate::contexts::agent_runtime::infrastructure::context_projection::project_request;
use std::collections::BTreeSet;

fn policy(version: &'static str, mutation: ContextPolicyMutation) -> ContextPolicyProfile {
    ContextPolicyProfile {
        version,
        target_basis_points: 9_000,
        mutation,
    }
}

#[test]
fn corpus_is_versioned_content_safe_and_covers_every_required_boundary() {
    let corpus = context_policy_regression_corpus();
    assert_eq!(
        CONTEXT_POLICY_CORPUS_VERSION,
        "onepiece-context-regression-corpus-v1"
    );
    assert_eq!(corpus.len(), 7);
    let ids = corpus.iter().map(|case| case.id).collect::<BTreeSet<_>>();
    assert_eq!(
        ids,
        BTreeSet::from([
            "arithmetic-boundary",
            "large-tool-result",
            "multilingual-sizes",
            "protocol-rounds",
            "reinjection",
            "retention-classes",
            "unavailable-tokens",
        ])
    );
    assert!(corpus
        .iter()
        .any(|case| case.adjustment == CorpusAdjustment::MarkReinjectable));
    assert!(corpus
        .iter()
        .any(|case| case.adjustment == CorpusAdjustment::TokensUnavailable));
    let retention_classes = corpus
        .iter()
        .flat_map(|case| {
            adjusted_snapshot(project_request(&case.body, case.shape), case.adjustment)
                .components
                .into_iter()
                .map(|component| component.retention_class)
        })
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(retention_classes.len(), 6);
    let serialized =
        serde_json::to_string(&corpus.iter().map(|case| &case.body).collect::<Vec<_>>())
            .expect("corpus");
    assert!(serialized.contains("fixture-"));
    for forbidden in [
        "private-prompt",
        "Authorization",
        "Bearer",
        "sk-sensitive",
        "D:\\\\",
    ] {
        assert!(!serialized.contains(forbidden));
    }
}

#[test]
fn identical_policy_evaluations_are_repeatable_and_baseline_safe() {
    let active = policy("active-v1", ContextPolicyMutation::None);
    let candidate = policy("candidate-v1", ContextPolicyMutation::None);
    let first = evaluate_context_policies(&active, &candidate).expect("first evaluation");
    let repeated = evaluate_context_policies(&active, &candidate).expect("repeat evaluation");

    assert_eq!(first, repeated);
    assert_eq!(first.active.len(), 7);
    assert_eq!(first.active_passed, 7);
    assert_eq!(first.candidate_passed, 7);
    assert_eq!(first.regressions, 0);
    assert_eq!(first.optimizer_version, "onepiece-context-optimizer-v1");
    assert_eq!(first.verifier_version, "onepiece-context-verifier-v1");
    assert_eq!(first.snapshot_version, "onepiece-context-snapshot-v1");
    assert!(first
        .active
        .iter()
        .any(|result| { result.case_id == "unavailable-tokens" && result.saved_tokens.is_none() }));
    assert!(first
        .active
        .iter()
        .any(|result| result.case_id == "reinjection" && result.passed));
    assert!(first
        .active
        .iter()
        .any(|result| { result.case_id == "arithmetic-boundary" && result.saved_characters > 0 }));
    assert!(!first.candidate_activated);
}

#[test]
fn invariant_failures_override_savings_and_flag_baseline_success_regressions() {
    let active = policy("active-v1", ContextPolicyMutation::None);
    let candidate = policy(
        "candidate-unsafe-retention-v1",
        ContextPolicyMutation::DropFirstProtected,
    );
    let report = evaluate_context_policies(&active, &candidate).expect("evaluation");

    assert_eq!(report.active_passed, 7);
    assert!(report.candidate_passed < report.active_passed);
    assert!(report.regressions > 0);
    assert!(report.candidate.iter().any(|result| {
        !result.passed
            && result.saved_characters > 0
            && result
                .invariant_failures
                .contains(&"protected-content-changed")
    }));
    assert!(!report.candidate_activated);
}

#[test]
fn protocol_regressions_and_aggregate_comparisons_remain_bounded_and_non_authoritative() {
    let report = evaluate_context_policies(
        &policy("active-v1", ContextPolicyMutation::None),
        &policy(
            "candidate-broken-protocol-v1",
            ContextPolicyMutation::BreakProtocol,
        ),
    )
    .expect("evaluation");

    assert!(report
        .candidate
        .iter()
        .any(|result| { result.invariant_failures.contains(&"protocol-incomplete") }));
    assert!(report.active_passed <= report.active.len() as u32);
    assert!(report.candidate_passed <= report.candidate.len() as u32);
    assert!(report.regressions <= report.active.len() as u32);
    assert!(
        report.active_saved_characters
            >= report
                .active
                .iter()
                .map(|result| result.saved_characters)
                .max()
                .unwrap_or(0)
    );
    assert!(!report.candidate_activated);
}
