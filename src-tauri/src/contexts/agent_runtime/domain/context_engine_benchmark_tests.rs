use super::*;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::time::Instant;

#[derive(Deserialize)]
struct Dataset {
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    name: String,
    budget: u64,
    relevant: Vec<String>,
    #[serde(default)]
    duplicates: bool,
    candidates: Vec<(String, String, u64, u16)>,
}

#[test]
fn synthetic_context_benchmark_meets_quality_and_operation_budgets() {
    let dataset: Dataset =
        serde_json::from_str(include_str!("fixtures/context-engine-benchmark.json"))
            .expect("benchmark dataset");
    let started = Instant::now();
    let mut relevant_selected = 0_usize;
    let mut selected_total = 0_usize;
    let mut relevant_total = 0_usize;
    let mut useful_tokens = 0_u64;
    let mut total_tokens = 0_u64;
    let mut duplicate_savings = 0_u64;
    let mut overflows = 0_usize;
    let mut operations = 0_usize;
    for case in &dataset.cases {
        let candidates = case
            .candidates
            .iter()
            .map(|(id, source, tokens, relevance)| {
                benchmark_candidate(id, source, *tokens, *relevance, case.duplicates)
            })
            .collect::<Vec<_>>();
        operations += candidates.len();
        let first =
            select_context(candidates.clone(), &benchmark_budget(case.budget)).expect("selection");
        let repeated =
            select_context(candidates, &benchmark_budget(case.budget)).expect("repeated selection");
        assert_eq!(
            first.selected, repeated.selected,
            "unstable case {}",
            case.name
        );
        assert!(first.occupied_tokens <= case.budget);
        overflows += usize::from(first.occupied_tokens > case.budget);
        duplicate_savings = duplicate_savings.saturating_add(first.duplicate_tokens_saved);
        relevant_total += case.relevant.len();
        for evidence in first.selected {
            selected_total += 1;
            total_tokens = total_tokens.saturating_add(evidence.candidate.token_estimate);
            if case.relevant.contains(&evidence.candidate.id)
                || (case.duplicates && evidence.candidate.id.starts_with("same-"))
            {
                relevant_selected += 1;
                useful_tokens = useful_tokens.saturating_add(evidence.candidate.token_estimate);
            }
        }
    }
    assert!(
        relevant_selected * 100 >= relevant_total * 85,
        "Recall@budget"
    );
    assert!(
        relevant_selected * 100 >= selected_total * 75,
        "Precision@budget"
    );
    assert!(
        useful_tokens * 100 >= total_tokens * 75,
        "useful-token ratio"
    );
    assert!(duplicate_savings >= 80);
    assert_eq!(overflows, 0);
    assert!(operations <= 32);
    eprintln!(
        "context benchmark ranking_latency={:?} operations={operations}",
        started.elapsed()
    );
}

fn benchmark_candidate(
    id: &str,
    source: &str,
    tokens: u64,
    relevance: u16,
    duplicates: bool,
) -> ContextCandidate {
    let source_kind = ContextSourceKind::parse(source).expect("source kind");
    ContextCandidate {
        id: id.to_string(),
        source_kind,
        source_ref: if duplicates {
            "src/shared.rs".to_string()
        } else {
            format!("src/{id}.rs")
        },
        content: format!("content-{id}"),
        range: ContextRange::new(1, 4),
        symbol: None,
        token_estimate: tokens,
        estimate_quality: EstimateQuality::Estimated,
        signals: CandidateSignals {
            explicitness: u16::from(source_kind == ContextSourceKind::ExplicitReference) * 100,
            semantic_relevance: relevance,
            symbol_relation: relevance,
            path_proximity: 50,
            freshness: 50,
            authority: 50,
        },
        redundancy_group: None,
        protected: source_kind == ContextSourceKind::ExplicitReference,
        fingerprint: if duplicates {
            "shared".to_string()
        } else {
            id.to_string()
        },
        provenance: BTreeSet::from([source_kind]),
    }
}

fn benchmark_budget(total: u64) -> ContextBudget {
    ContextBudget {
        total,
        reserved_system: 0,
        reserved_task: 0,
        reserved_recent_turns: 0,
        reserve: 0,
    }
}
