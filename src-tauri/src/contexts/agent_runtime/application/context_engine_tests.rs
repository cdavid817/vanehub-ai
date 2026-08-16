use std::collections::BTreeSet;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use super::*;
use crate::contexts::agent_runtime::domain::{CandidateSignals, ContextRange, EstimateQuality};

struct Source(
    ContextSourceKind,
    ContextSourceOutcome,
    Vec<ContextCandidate>,
);

impl ContextCandidateSource for Source {
    fn kind(&self) -> ContextSourceKind {
        self.0
    }

    fn collect(&self, _request: &ContextRequest, _cancelled: &AtomicBool) -> ContextSourceResult {
        ContextSourceResult {
            outcome: self.1,
            candidates: self.2.clone(),
        }
    }
}

#[derive(Default)]
struct Repository(Mutex<Vec<ContextEvidenceManifest>>);

impl ContextManifestRepository for Repository {
    fn save(&self, manifest: &ContextEvidenceManifest) -> Result<(), String> {
        self.0.lock().expect("manifest lock").push(manifest.clone());
        Ok(())
    }

    fn list(
        &self,
        _session_id: Option<&str>,
        _cursor: Option<&str>,
        _limit: u32,
    ) -> Result<crate::contexts::agent_runtime::domain::ContextEvidenceManifestPage, String> {
        Ok(
            crate::contexts::agent_runtime::domain::ContextEvidenceManifestPage {
                items: self.0.lock().expect("saved lock").clone(),
                next_cursor: None,
            },
        )
    }

    fn get(&self, generation_id: &str) -> Result<Option<ContextEvidenceManifest>, String> {
        Ok(self
            .0
            .lock()
            .expect("saved lock")
            .iter()
            .find(|manifest| manifest.generation_id == generation_id)
            .cloned())
    }
}

#[derive(Default)]
struct Diagnostics(Mutex<Vec<ContextEngineDiagnostic>>);

impl ContextEngineDiagnosticPort for Diagnostics {
    fn record(&self, event: ContextEngineDiagnostic) {
        self.0.lock().expect("diagnostic lock").push(event);
    }
}

struct Clock;

impl ContextEngineClockPort for Clock {
    fn now_millis(&self) -> u64 {
        42
    }
}

struct FailingRepository;
impl ContextManifestRepository for FailingRepository {
    fn save(&self, _manifest: &ContextEvidenceManifest) -> Result<(), String> {
        Err("database contains secret source body".to_string())
    }
    fn list(
        &self,
        _session_id: Option<&str>,
        _cursor: Option<&str>,
        _limit: u32,
    ) -> Result<crate::contexts::agent_runtime::domain::ContextEvidenceManifestPage, String> {
        Err("unavailable".to_string())
    }
    fn get(&self, _generation_id: &str) -> Result<Option<ContextEvidenceManifest>, String> {
        Err("unavailable".to_string())
    }
}

fn candidate(id: &str, kind: ContextSourceKind) -> ContextCandidate {
    ContextCandidate {
        id: id.to_string(),
        source_kind: kind,
        source_ref: "src/example.rs".to_string(),
        content: format!("safe provider content {id}"),
        range: ContextRange::new(4, 8),
        symbol: Some("example".to_string()),
        token_estimate: 20,
        estimate_quality: EstimateQuality::Estimated,
        signals: CandidateSignals {
            explicitness: 0,
            semantic_relevance: 80,
            symbol_relation: 50,
            path_proximity: 20,
            freshness: 10,
            authority: 0,
        },
        redundancy_group: None,
        protected: false,
        fingerprint: format!("safe-{id}"),
        provenance: BTreeSet::new(),
    }
}

fn request() -> ContextRequest {
    ContextRequest {
        session_id: "session-1".to_string(),
        turn_id: "turn-1".to_string(),
        generation_id: "generation-1".to_string(),
        task: "fix example".to_string(),
        workspace_ref: Some("workspace-1".to_string()),
        explicit_refs: Vec::new(),
        model_capacity: Some(1_000),
    }
}

fn budget() -> ContextBudget {
    ContextBudget {
        total: 100,
        reserved_system: 10,
        reserved_task: 10,
        reserved_recent_turns: 10,
        reserve: 10,
    }
}

#[test]
fn unavailable_lsp_does_not_remove_retrieval_and_tree_sitter_candidates() {
    let repository = Arc::new(Repository::default());
    let diagnostics = Arc::new(Diagnostics::default());
    let service = ContextEngineService::new(
        vec![
            Arc::new(Source(
                ContextSourceKind::LspDefinition,
                ContextSourceOutcome::Unavailable,
                Vec::new(),
            )),
            Arc::new(Source(
                ContextSourceKind::Retrieval,
                ContextSourceOutcome::Ready,
                vec![candidate("retrieval", ContextSourceKind::Retrieval)],
            )),
            Arc::new(Source(
                ContextSourceKind::TreeSitter,
                ContextSourceOutcome::Ready,
                vec![candidate("tree", ContextSourceKind::TreeSitter)],
            )),
        ],
        repository.clone(),
        diagnostics,
        Arc::new(Clock),
    );

    let ContextEngineOutcome::Ready(output) =
        service.assemble(&request(), &budget(), &AtomicBool::new(false))
    else {
        panic!("expected ready output");
    };
    assert_eq!(output.manifest.selected.len(), 2);
    assert_eq!(
        output
            .manifest
            .source_outcomes
            .get(&ContextSourceKind::LspDefinition),
        Some(&ContextSourceOutcome::Unavailable)
    );
}

#[test]
fn projection_has_bounded_labels_and_content_but_manifest_has_no_content() {
    let repository = Arc::new(Repository::default());
    let service = ContextEngineService::new(
        vec![Arc::new(Source(
            ContextSourceKind::Retrieval,
            ContextSourceOutcome::Ready,
            vec![candidate("definition", ContextSourceKind::Retrieval)],
        ))],
        repository.clone(),
        Arc::new(Diagnostics::default()),
        Arc::new(Clock),
    );
    let ContextEngineOutcome::Ready(output) =
        service.assemble(&request(), &budget(), &AtomicBool::new(false))
    else {
        panic!("expected ready output");
    };
    assert!(output.provider_projection.contains("source=Retrieval"));
    assert!(output.provider_projection.contains("safe provider content"));
    let persisted = format!("{:?}", repository.0.lock().expect("manifest lock"));
    assert!(!persisted.contains("safe provider content"));
}

#[test]
fn cancellation_and_protected_overflow_use_unchanged_safe_fallback() {
    let service = ContextEngineService::new(
        Vec::new(),
        Arc::new(Repository::default()),
        Arc::new(Diagnostics::default()),
        Arc::new(Clock),
    );
    assert_eq!(
        service.assemble(&request(), &budget(), &AtomicBool::new(true)),
        ContextEngineOutcome::SafeFallback("cancelled")
    );

    let mut protected = candidate("explicit", ContextSourceKind::ExplicitReference);
    protected.protected = true;
    protected.token_estimate = 1_000;
    let service = ContextEngineService::new(
        vec![Arc::new(Source(
            ContextSourceKind::ExplicitReference,
            ContextSourceOutcome::Ready,
            vec![protected],
        ))],
        Arc::new(Repository::default()),
        Arc::new(Diagnostics::default()),
        Arc::new(Clock),
    );
    assert_eq!(
        service.assemble(&request(), &budget(), &AtomicBool::new(false)),
        ContextEngineOutcome::SafeFallback("protected-overflow")
    );
}

#[test]
fn persistence_failure_does_not_change_successful_provider_projection() {
    let source = Arc::new(Source(
        ContextSourceKind::Retrieval,
        ContextSourceOutcome::Ready,
        vec![candidate("safe", ContextSourceKind::Retrieval)],
    ));
    let service = ContextEngineService::new(
        vec![source],
        Arc::new(FailingRepository),
        Arc::new(Diagnostics::default()),
        Arc::new(Clock),
    );
    let result = service.assemble(&request(), &budget(), &AtomicBool::new(false));
    assert!(matches!(result, ContextEngineOutcome::Ready(_)));
}

#[test]
fn every_optional_degradation_state_is_preserved_without_failing_selection() {
    for outcome in [
        ContextSourceOutcome::Warming,
        ContextSourceOutcome::Unavailable,
        ContextSourceOutcome::TimedOut,
        ContextSourceOutcome::Failed,
        ContextSourceOutcome::Cancelled,
    ] {
        let service = ContextEngineService::new(
            vec![
                Arc::new(Source(ContextSourceKind::LspReference, outcome, Vec::new())),
                Arc::new(Source(
                    ContextSourceKind::TreeSitter,
                    ContextSourceOutcome::Ready,
                    vec![candidate("fallback", ContextSourceKind::TreeSitter)],
                )),
            ],
            Arc::new(Repository::default()),
            Arc::new(Diagnostics::default()),
            Arc::new(Clock),
        );
        let ContextEngineOutcome::Ready(result) =
            service.assemble(&request(), &budget(), &AtomicBool::new(false))
        else {
            panic!("optional source must not fail selection");
        };
        assert_eq!(
            result
                .manifest
                .selected
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["fallback"]
        );
        assert_eq!(
            result
                .manifest
                .source_outcomes
                .get(&ContextSourceKind::LspReference),
            Some(&outcome)
        );
    }
}
