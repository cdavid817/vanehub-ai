use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::contexts::agent_runtime::domain::{
    select_context, ContextBudget, ContextCandidate, ContextEvidenceManifest,
    ContextEvidenceManifestPage, ContextEvidenceSummary, ContextReasonCode, ContextRequest,
    ContextSelectionError, ContextSourceKind, ContextSourceOutcome,
};

const MAX_SOURCES: usize = 16;
const MAX_CANDIDATES_PER_SOURCE: usize = 64;
const MAX_REJECTED: usize = 16;
const MAX_COLLECTION_MILLIS: u64 = 2_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextPlan {
    pub(crate) source_kinds: Vec<ContextSourceKind>,
    pub(crate) evidence_budget: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextSourceResult {
    pub(crate) outcome: ContextSourceOutcome,
    pub(crate) candidates: Vec<ContextCandidate>,
}

pub(crate) trait ContextCandidateSource: Send + Sync {
    fn kind(&self) -> ContextSourceKind;
    fn collect(&self, request: &ContextRequest, cancelled: &AtomicBool) -> ContextSourceResult;
}

pub(crate) trait ContextManifestRepository: Send + Sync {
    fn save(&self, manifest: &ContextEvidenceManifest) -> Result<(), String>;
    fn list(
        &self,
        session_id: Option<&str>,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<ContextEvidenceManifestPage, String>;
    fn get(&self, generation_id: &str) -> Result<Option<ContextEvidenceManifest>, String>;
}

pub(crate) trait ContextEngineDiagnosticPort: Send + Sync {
    fn record(&self, event: ContextEngineDiagnostic);
}

pub(crate) trait ContextEngineClockPort: Send + Sync {
    fn now_millis(&self) -> u64;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextEngineDiagnostic {
    pub(crate) policy_version: &'static str,
    pub(crate) selected_count: usize,
    pub(crate) rejected_count: usize,
    pub(crate) occupied_tokens: u64,
    pub(crate) outcome: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectedContextEvidence {
    pub(crate) manifest: ContextEvidenceManifest,
    pub(crate) provider_projection: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ContextEngineOutcome {
    Ready(Box<ProjectedContextEvidence>),
    SafeFallback(&'static str),
}

pub(crate) struct ContextEngineService {
    sources: Vec<Arc<dyn ContextCandidateSource>>,
    manifests: Arc<dyn ContextManifestRepository>,
    diagnostics: Arc<dyn ContextEngineDiagnosticPort>,
    clock: Arc<dyn ContextEngineClockPort>,
}

impl ContextEngineService {
    pub(crate) fn new(
        sources: Vec<Arc<dyn ContextCandidateSource>>,
        manifests: Arc<dyn ContextManifestRepository>,
        diagnostics: Arc<dyn ContextEngineDiagnosticPort>,
        clock: Arc<dyn ContextEngineClockPort>,
    ) -> Self {
        Self {
            sources: sources.into_iter().take(MAX_SOURCES).collect(),
            manifests,
            diagnostics,
            clock,
        }
    }

    pub(crate) fn assemble(
        &self,
        request: &ContextRequest,
        budget: &ContextBudget,
        cancelled: &AtomicBool,
    ) -> ContextEngineOutcome {
        if cancelled.load(Ordering::Relaxed) {
            return ContextEngineOutcome::SafeFallback("cancelled");
        }
        let started = self.clock.now_millis();
        let plan = self.plan(request, budget);
        let mut outcomes = BTreeMap::new();
        let mut candidates = Vec::new();
        for source in &self.sources {
            if !plan.source_kinds.contains(&source.kind()) {
                continue;
            }
            if cancelled.load(Ordering::Relaxed) {
                return ContextEngineOutcome::SafeFallback("cancelled");
            }
            let mut result = source.collect(request, cancelled);
            if self.clock.now_millis().saturating_sub(started) > MAX_COLLECTION_MILLIS {
                result.outcome = ContextSourceOutcome::TimedOut;
                result.candidates.clear();
            }
            result.candidates.truncate(MAX_CANDIDATES_PER_SOURCE);
            if result.outcome != ContextSourceOutcome::Ready {
                self.diagnostics.record(ContextEngineDiagnostic {
                    policy_version:
                        crate::contexts::agent_runtime::domain::CONTEXT_ENGINE_POLICY_VERSION,
                    selected_count: 0,
                    rejected_count: 0,
                    occupied_tokens: 0,
                    outcome: "source-degraded",
                });
            }
            outcomes.insert(source.kind(), result.outcome);
            candidates.extend(result.candidates);
        }

        let selection = match select_context(candidates, budget) {
            Ok(selection) => selection,
            Err(ContextSelectionError::ProtectedOverflow { .. }) => {
                self.diagnostics.record(ContextEngineDiagnostic {
                    policy_version:
                        crate::contexts::agent_runtime::domain::CONTEXT_ENGINE_POLICY_VERSION,
                    selected_count: 0,
                    rejected_count: 0,
                    occupied_tokens: 0,
                    outcome: "protected-overflow",
                });
                return ContextEngineOutcome::SafeFallback("protected-overflow");
            }
        };
        let provider_projection = project(&selection.selected);
        let manifest = ContextEvidenceManifest {
            session_id: request.session_id.clone(),
            turn_id: request.turn_id.clone(),
            generation_id: request.generation_id.clone(),
            recorded_at: self.clock.now_millis().to_string(),
            policy_version: selection.policy_version.to_string(),
            evidence_budget: budget.evidence_budget(),
            occupied_tokens: selection.occupied_tokens,
            selected: selection
                .selected
                .iter()
                .map(|item| ContextEvidenceSummary {
                    id: item.candidate.id.clone(),
                    source_kind: item.candidate.source_kind,
                    source_ref: item.candidate.source_ref.clone(),
                    range: item.candidate.range,
                    symbol: item.candidate.symbol.clone(),
                    token_estimate: item.candidate.token_estimate,
                    safe_fingerprint: item.candidate.fingerprint.clone(),
                    reasons: item.reasons.clone(),
                })
                .collect(),
            rejected: selection.rejected.into_iter().take(MAX_REJECTED).collect(),
            source_outcomes: outcomes,
            duplicate_tokens_saved: selection.duplicate_tokens_saved,
            collection_latency_bucket: latency_bucket(
                self.clock.now_millis().saturating_sub(started),
            )
            .to_string(),
            ranking_latency_bucket: "sub-10ms".to_string(),
            compaction_triggered: false,
        };
        let _elapsed = self.clock.now_millis().saturating_sub(started);
        if self.manifests.save(&manifest).is_err() {
            self.diagnostics.record(ContextEngineDiagnostic {
                policy_version: selection.policy_version,
                selected_count: manifest.selected.len(),
                rejected_count: manifest.rejected.len(),
                occupied_tokens: manifest.occupied_tokens,
                outcome: "persistence-failed",
            });
        }
        self.diagnostics.record(ContextEngineDiagnostic {
            policy_version: selection.policy_version,
            selected_count: manifest.selected.len(),
            rejected_count: manifest.rejected.len(),
            occupied_tokens: manifest.occupied_tokens,
            outcome: "ready",
        });
        ContextEngineOutcome::Ready(Box::new(ProjectedContextEvidence {
            manifest,
            provider_projection,
        }))
    }

    pub(crate) fn plan(&self, request: &ContextRequest, budget: &ContextBudget) -> ContextPlan {
        let source_kinds = self
            .sources
            .iter()
            .map(|source| source.kind())
            .filter(|kind| {
                request.workspace_ref.is_some()
                    || matches!(
                        kind,
                        ContextSourceKind::Memory | ContextSourceKind::AuthoritativeState
                    )
            })
            .collect();
        ContextPlan {
            source_kinds,
            evidence_budget: budget.evidence_budget(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct ContextManifestQueryService {
    repository: Arc<dyn ContextManifestRepository>,
}

impl ContextManifestQueryService {
    pub(crate) fn new(repository: Arc<dyn ContextManifestRepository>) -> Self {
        Self { repository }
    }

    pub(crate) fn list(
        &self,
        session_id: Option<&str>,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<ContextEvidenceManifestPage, String> {
        self.repository
            .list(session_id, cursor, limit.unwrap_or(20))
    }

    pub(crate) fn get(
        &self,
        generation_id: &str,
    ) -> Result<Option<ContextEvidenceManifest>, String> {
        self.repository.get(generation_id)
    }
}

fn latency_bucket(millis: u64) -> &'static str {
    match millis {
        0..=9 => "sub-10ms",
        10..=49 => "10-49ms",
        50..=249 => "50-249ms",
        _ => "250ms-plus",
    }
}

fn project(selected: &[crate::contexts::agent_runtime::domain::ContextEvidence]) -> String {
    selected
        .iter()
        .map(|evidence| {
            let range = evidence
                .candidate
                .range
                .map(|range| format!("{}-{}", range.start_line, range.end_line))
                .unwrap_or_else(|| "whole".to_string());
            let reasons = evidence
                .reasons
                .iter()
                .map(reason_label)
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "[context source={:?} ref={} range={} reasons={}]\n{}",
                evidence.candidate.source_kind,
                evidence.candidate.source_ref,
                range,
                reasons,
                evidence.candidate.content
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn reason_label(reason: &ContextReasonCode) -> &'static str {
    match reason {
        ContextReasonCode::Explicit => "explicit",
        ContextReasonCode::SemanticMatch => "semantic-match",
        ContextReasonCode::SymbolRelation => "symbol-relation",
        ContextReasonCode::PathProximity => "path-proximity",
        ContextReasonCode::Fresh => "fresh",
        ContextReasonCode::Authoritative => "authoritative",
        ContextReasonCode::DuplicateMerged => "duplicate-merged",
        ContextReasonCode::BudgetSelected => "budget-selected",
        ContextReasonCode::BudgetRejected => "budget-rejected",
        ContextReasonCode::SourceUnavailable => "source-unavailable",
        ContextReasonCode::UnsafeProvenance => "unsafe-provenance",
    }
}

#[cfg(test)]
#[path = "context_engine_tests.rs"]
mod tests;
