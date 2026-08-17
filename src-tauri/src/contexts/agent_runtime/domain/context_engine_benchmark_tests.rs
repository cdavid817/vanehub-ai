use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::time::Instant;

#[derive(Deserialize)]
struct Dataset {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    #[serde(rename = "datasetId")]
    dataset_id: String,
    #[serde(rename = "datasetVersion")]
    dataset_version: u32,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ContextPerformanceEvidence<'a> {
    schema_version: u32,
    dataset_id: &'a str,
    dataset_version: u32,
    policy_version: &'a str,
    phases: PhaseOperations,
    candidate_count: usize,
    selected_count: usize,
    selected_bytes: usize,
    occupied_tokens: u64,
    useful_tokens: u64,
    duplicate_tokens_saved: u64,
    overflow_count: usize,
    measurement_quality: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PhaseOperations {
    collection: usize,
    ranking: usize,
    deduplication: usize,
    budgeting: usize,
    projection: usize,
    index_queries: usize,
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
    let mut selected_bytes = 0_usize;
    let mut duplicate_savings = 0_u64;
    let mut overflows = 0_usize;
    let mut operations = 0_usize;
    let mut index_queries = 0_usize;
    assert_eq!(dataset.schema_version, 1);
    assert_eq!(dataset.dataset_id, "repo-small");
    assert_eq!(dataset.dataset_version, 1);
    for case in &dataset.cases {
        let candidates = case
            .candidates
            .iter()
            .map(|(id, source, tokens, relevance)| {
                benchmark_candidate(id, source, *tokens, *relevance, case.duplicates)
            })
            .collect::<Vec<_>>();
        operations += candidates.len();
        index_queries += candidates
            .iter()
            .filter(|candidate| {
                matches!(
                    candidate.source_kind,
                    ContextSourceKind::Retrieval
                        | ContextSourceKind::LspDefinition
                        | ContextSourceKind::LspReference
                )
            })
            .count();
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
        for evidence in &first.selected {
            selected_total += 1;
            total_tokens = total_tokens.saturating_add(evidence.candidate.token_estimate);
            selected_bytes = selected_bytes.saturating_add(evidence.candidate.content.len());
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
    let evidence = ContextPerformanceEvidence {
        schema_version: dataset.schema_version,
        dataset_id: &dataset.dataset_id,
        dataset_version: dataset.dataset_version,
        policy_version: CONTEXT_ENGINE_POLICY_VERSION,
        phases: PhaseOperations {
            collection: operations,
            ranking: operations,
            deduplication: operations,
            budgeting: operations,
            projection: selected_total,
            index_queries,
        },
        candidate_count: operations,
        selected_count: selected_total,
        selected_bytes,
        occupied_tokens: total_tokens,
        useful_tokens,
        duplicate_tokens_saved: duplicate_savings,
        overflow_count: overflows,
        measurement_quality: "estimated",
    };
    let encoded = serde_json::to_string(&evidence).expect("performance evidence");
    assert!(!encoded.contains("content-"));
    assert!(!encoded.contains("src/"));
    assert!(!encoded.contains("prompt"));
    eprintln!("CONTEXT_PERFORMANCE {encoded}");
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
