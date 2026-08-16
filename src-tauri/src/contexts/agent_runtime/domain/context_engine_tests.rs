use std::collections::BTreeSet;

use super::{
    select_context, CandidateSignals, ContextBudget, ContextCandidate, ContextRange,
    ContextSelectionError, ContextSourceKind, EstimateQuality,
};
use crate::contexts::agent_runtime::domain::{
    classify_components, ContextComponent, ContextRound, ProtocolState, RetentionClass,
    SemanticClass,
};

fn candidate(id: &str, source: ContextSourceKind, tokens: u64) -> ContextCandidate {
    ContextCandidate {
        id: id.to_string(),
        source_kind: source,
        source_ref: "src/lib.rs".to_string(),
        content: format!("content-{id}"),
        range: ContextRange::new(1, 4),
        symbol: Some(id.to_string()),
        token_estimate: tokens,
        estimate_quality: EstimateQuality::Estimated,
        signals: CandidateSignals {
            explicitness: 0,
            semantic_relevance: 50,
            symbol_relation: 0,
            path_proximity: 0,
            freshness: 0,
            authority: 0,
        },
        redundancy_group: None,
        protected: false,
        fingerprint: format!("fp-{id}"),
        provenance: BTreeSet::new(),
    }
}

fn budget(tokens: u64) -> ContextBudget {
    ContextBudget {
        total: tokens,
        reserved_system: 0,
        reserved_task: 0,
        reserved_recent_turns: 0,
        reserve: 0,
    }
}

#[test]
fn explicit_reference_is_protected_from_higher_optional_score() {
    let mut explicit = candidate("explicit", ContextSourceKind::ExplicitReference, 80);
    explicit.protected = true;
    explicit.signals.explicitness = 100;
    let mut optional = candidate("optional", ContextSourceKind::Retrieval, 30);
    optional.signals.semantic_relevance = 100;

    let result = select_context(vec![optional, explicit], &budget(80)).expect("selection");
    assert_eq!(result.selected.len(), 1);
    assert_eq!(result.selected[0].candidate.id, "explicit");
}

#[test]
fn ranking_is_deterministic_with_stable_id_tie_breaking() {
    let first = select_context(
        vec![
            candidate("b", ContextSourceKind::Retrieval, 10),
            candidate("a", ContextSourceKind::TreeSitter, 10),
        ],
        &budget(20),
    )
    .expect("first");
    let second = select_context(
        vec![
            candidate("a", ContextSourceKind::TreeSitter, 10),
            candidate("b", ContextSourceKind::Retrieval, 10),
        ],
        &budget(20),
    )
    .expect("second");
    let ids = |selection: &super::ContextSelection| {
        selection
            .selected
            .iter()
            .map(|item| item.candidate.id.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&first), vec!["a".to_string(), "b".to_string()]);
    assert_eq!(ids(&first), ids(&second));
}

#[test]
fn definition_tests_and_callers_share_one_ranked_pipeline() {
    let mut definition = candidate("definition", ContextSourceKind::LspDefinition, 20);
    definition.signals.symbol_relation = 100;
    let mut caller = candidate("caller", ContextSourceKind::LspReference, 20);
    caller.signals.symbol_relation = 80;
    let mut test = candidate("test", ContextSourceKind::Test, 20);
    test.signals.semantic_relevance = 90;

    let result = select_context(vec![test, caller, definition], &budget(60)).expect("selection");
    assert_eq!(result.selected.len(), 3);
    assert_eq!(result.selected[0].candidate.id, "definition");
}

#[test]
fn three_sources_with_one_fingerprint_charge_once() {
    let mut retrieval = candidate("retrieval", ContextSourceKind::Retrieval, 30);
    let mut tree = candidate("tree", ContextSourceKind::TreeSitter, 30);
    let mut lsp = candidate("lsp", ContextSourceKind::LspDefinition, 30);
    for item in [&mut retrieval, &mut tree, &mut lsp] {
        item.fingerprint = "same-safe-fingerprint".to_string();
    }

    let result = select_context(vec![retrieval, tree, lsp], &budget(30)).expect("selection");
    assert_eq!(result.selected.len(), 1);
    assert_eq!(result.selected[0].candidate.provenance.len(), 3);
    assert_eq!(result.duplicate_tokens_saved, 60);
}

#[test]
fn budget_rejects_low_value_candidate_without_partial_range() {
    let mut high = candidate("high", ContextSourceKind::Retrieval, 40);
    high.signals.semantic_relevance = 100;
    let low = candidate("low", ContextSourceKind::Memory, 40);
    let result = select_context(vec![low, high], &budget(50)).expect("selection");
    assert_eq!(result.selected[0].candidate.range, ContextRange::new(1, 4));
    assert_eq!(result.rejected.len(), 1);
    assert_eq!(result.occupied_tokens, 40);
}

#[test]
fn malformed_candidate_is_rejected_and_protected_overflow_is_typed() {
    let mut unsafe_candidate = candidate("unsafe", ContextSourceKind::Retrieval, 10);
    unsafe_candidate.source_ref = "../secret".to_string();
    let result = select_context(vec![unsafe_candidate], &budget(10)).expect("selection");
    assert!(result.selected.is_empty());

    let mut protected = candidate("protected", ContextSourceKind::ExplicitReference, u64::MAX);
    protected.protected = true;
    assert!(matches!(
        select_context(vec![protected], &budget(100)),
        Err(ContextSelectionError::ProtectedOverflow {
            required: u64::MAX,
            available: 100
        })
    ));
}

#[test]
fn reserved_budget_uses_saturating_arithmetic() {
    let budget = ContextBudget {
        total: 10,
        reserved_system: u64::MAX,
        reserved_task: u64::MAX,
        reserved_recent_turns: u64::MAX,
        reserve: u64::MAX,
    };
    assert_eq!(budget.evidence_budget(), 0);
}

#[test]
fn projected_evidence_has_distinct_measurement_class_and_complete_boundary() {
    let mut components = vec![ContextComponent {
        sequence: 0,
        semantic_class: SemanticClass::ContextEvidence,
        retention_class: RetentionClass::Discardable,
        round: Some(0),
        characters: 40,
        estimated_tokens: Some(10),
        content_fingerprint: "evidence-fingerprint".to_string(),
        tool_reference: None,
        current_user_intent: false,
        correction: false,
        reinjectable: false,
        repeated_tool_result: false,
    }];
    classify_components(
        &mut components,
        &[ContextRound {
            index: 0,
            protocol_state: ProtocolState::Complete,
            component_sequences: vec![0],
        }],
    );
    assert_eq!(components[0].semantic_class, SemanticClass::ContextEvidence);
    assert_eq!(components[0].retention_class, RetentionClass::Verbatim);
}

#[test]
fn memory_has_an_independent_source_budget() {
    let mut memory_a = candidate("memory-a", ContextSourceKind::Memory, 20);
    let mut memory_b = candidate("memory-b", ContextSourceKind::Memory, 20);
    memory_a.signals.semantic_relevance = 100;
    memory_b.signals.semantic_relevance = 90;
    let code = candidate("code", ContextSourceKind::Retrieval, 60);
    let result = select_context(vec![memory_a, memory_b, code], &budget(100)).expect("selection");
    assert_eq!(
        result
            .selected
            .iter()
            .filter(|item| item.candidate.source_kind == ContextSourceKind::Memory)
            .count(),
        1
    );
    assert!(result.occupied_tokens <= 100);
}
