use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const CONTEXT_ENGINE_POLICY_VERSION: &str = "context-engine-v1";

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ContextSourceKind {
    ExplicitReference,
    Retrieval,
    TreeSitter,
    LspDefinition,
    LspReference,
    Test,
    WorkspaceChange,
    Memory,
    AuthoritativeState,
}

impl ContextSourceKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitReference => "explicit-reference",
            Self::Retrieval => "retrieval",
            Self::TreeSitter => "tree-sitter",
            Self::LspDefinition => "lsp-definition",
            Self::LspReference => "lsp-reference",
            Self::Test => "test",
            Self::WorkspaceChange => "workspace-change",
            Self::Memory => "memory",
            Self::AuthoritativeState => "authoritative-state",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "explicit-reference" => Self::ExplicitReference,
            "retrieval" => Self::Retrieval,
            "tree-sitter" => Self::TreeSitter,
            "lsp-definition" => Self::LspDefinition,
            "lsp-reference" => Self::LspReference,
            "test" => Self::Test,
            "workspace-change" => Self::WorkspaceChange,
            "memory" => Self::Memory,
            "authoritative-state" => Self::AuthoritativeState,
            _ => return None,
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EstimateQuality {
    Estimated,
    CharactersOnly,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContextReasonCode {
    Explicit,
    SemanticMatch,
    SymbolRelation,
    PathProximity,
    Fresh,
    Authoritative,
    DuplicateMerged,
    BudgetSelected,
    BudgetRejected,
    SourceUnavailable,
    UnsafeProvenance,
}

impl ContextReasonCode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::SemanticMatch => "semantic-match",
            Self::SymbolRelation => "symbol-relation",
            Self::PathProximity => "path-proximity",
            Self::Fresh => "fresh",
            Self::Authoritative => "authoritative",
            Self::DuplicateMerged => "duplicate-merged",
            Self::BudgetSelected => "budget-selected",
            Self::BudgetRejected => "budget-rejected",
            Self::SourceUnavailable => "source-unavailable",
            Self::UnsafeProvenance => "unsafe-provenance",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "explicit" => Self::Explicit,
            "semantic-match" => Self::SemanticMatch,
            "symbol-relation" => Self::SymbolRelation,
            "path-proximity" => Self::PathProximity,
            "fresh" => Self::Fresh,
            "authoritative" => Self::Authoritative,
            "duplicate-merged" => Self::DuplicateMerged,
            "budget-selected" => Self::BudgetSelected,
            "budget-rejected" => Self::BudgetRejected,
            "source-unavailable" => Self::SourceUnavailable,
            "unsafe-provenance" => Self::UnsafeProvenance,
            _ => return None,
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextRequest {
    pub(crate) session_id: String,
    pub(crate) turn_id: String,
    pub(crate) generation_id: String,
    pub(crate) task: String,
    pub(crate) workspace_ref: Option<String>,
    pub(crate) explicit_refs: Vec<String>,
    pub(crate) model_capacity: Option<u64>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContextSourceOutcome {
    Ready,
    Warming,
    Unavailable,
    TimedOut,
    Failed,
    Cancelled,
}

impl ContextSourceOutcome {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Warming => "warming",
            Self::Unavailable => "unavailable",
            Self::TimedOut => "timed-out",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
    pub(crate) fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "ready" => Self::Ready,
            "warming" => Self::Warming,
            "unavailable" => Self::Unavailable,
            "timed-out" => Self::TimedOut,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ContextRange {
    pub(crate) start_line: u32,
    pub(crate) end_line: u32,
}

impl ContextRange {
    pub(crate) fn new(start_line: u32, end_line: u32) -> Option<Self> {
        (start_line > 0 && end_line >= start_line).then_some(Self {
            start_line,
            end_line,
        })
    }

    fn overlaps(self, other: Self) -> bool {
        self.start_line <= other.end_line.saturating_add(1)
            && other.start_line <= self.end_line.saturating_add(1)
    }

    fn merge(self, other: Self) -> Self {
        Self {
            start_line: self.start_line.min(other.start_line),
            end_line: self.end_line.max(other.end_line),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CandidateSignals {
    pub(crate) explicitness: u16,
    pub(crate) semantic_relevance: u16,
    pub(crate) symbol_relation: u16,
    pub(crate) path_proximity: u16,
    pub(crate) freshness: u16,
    pub(crate) authority: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextCandidate {
    pub(crate) id: String,
    pub(crate) source_kind: ContextSourceKind,
    pub(crate) source_ref: String,
    pub(crate) content: String,
    pub(crate) range: Option<ContextRange>,
    pub(crate) symbol: Option<String>,
    pub(crate) token_estimate: u64,
    pub(crate) estimate_quality: EstimateQuality,
    pub(crate) signals: CandidateSignals,
    pub(crate) redundancy_group: Option<String>,
    pub(crate) protected: bool,
    pub(crate) fingerprint: String,
    pub(crate) provenance: BTreeSet<ContextSourceKind>,
}

impl ContextCandidate {
    pub(crate) fn is_valid(&self) -> bool {
        !self.id.is_empty()
            && !self.source_ref.is_empty()
            && !self.fingerprint.is_empty()
            && self.token_estimate > 0
            && !self.source_ref.starts_with('/')
            && !self.source_ref.split('/').any(|part| part == "..")
            && self.signals.values().iter().all(|value| *value <= 100)
    }

    pub(crate) fn score(&self) -> i64 {
        let value = i64::from(self.signals.explicitness) * 12
            + i64::from(self.signals.semantic_relevance) * 7
            + i64::from(self.signals.symbol_relation) * 6
            + i64::from(self.signals.path_proximity) * 3
            + i64::from(self.signals.freshness) * 2
            + i64::from(self.signals.authority) * 8;
        value - i64::try_from(self.token_estimate / 64).unwrap_or(i64::MAX)
    }
}

impl CandidateSignals {
    fn values(&self) -> [u16; 6] {
        [
            self.explicitness,
            self.semantic_relevance,
            self.symbol_relation,
            self.path_proximity,
            self.freshness,
            self.authority,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextBudget {
    pub(crate) total: u64,
    pub(crate) reserved_system: u64,
    pub(crate) reserved_task: u64,
    pub(crate) reserved_recent_turns: u64,
    pub(crate) reserve: u64,
}

impl ContextBudget {
    pub(crate) fn evidence_budget(&self) -> u64 {
        self.total
            .saturating_sub(self.reserved_system)
            .saturating_sub(self.reserved_task)
            .saturating_sub(self.reserved_recent_turns)
            .saturating_sub(self.reserve)
    }

    fn source_limit(&self, kind: ContextSourceKind) -> u64 {
        let available = self.evidence_budget();
        match kind {
            ContextSourceKind::ExplicitReference | ContextSourceKind::AuthoritativeState => {
                available
            }
            ContextSourceKind::Memory => available / 4,
            ContextSourceKind::WorkspaceChange => available / 5,
            _ => available,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextEvidence {
    pub(crate) candidate: ContextCandidate,
    pub(crate) score: i64,
    pub(crate) reasons: Vec<ContextReasonCode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextSelection {
    pub(crate) policy_version: &'static str,
    pub(crate) selected: Vec<ContextEvidence>,
    pub(crate) rejected: Vec<(String, ContextReasonCode)>,
    pub(crate) occupied_tokens: u64,
    pub(crate) duplicate_tokens_saved: u64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextEvidenceManifest {
    pub(crate) session_id: String,
    pub(crate) turn_id: String,
    pub(crate) generation_id: String,
    pub(crate) recorded_at: String,
    pub(crate) policy_version: String,
    pub(crate) evidence_budget: u64,
    pub(crate) occupied_tokens: u64,
    pub(crate) selected: Vec<ContextEvidenceSummary>,
    pub(crate) rejected: Vec<(String, ContextReasonCode)>,
    pub(crate) source_outcomes: BTreeMap<ContextSourceKind, ContextSourceOutcome>,
    pub(crate) duplicate_tokens_saved: u64,
    pub(crate) collection_latency_bucket: String,
    pub(crate) ranking_latency_bucket: String,
    pub(crate) compaction_triggered: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextEvidenceSummary {
    pub(crate) id: String,
    pub(crate) source_kind: ContextSourceKind,
    pub(crate) source_ref: String,
    pub(crate) range: Option<ContextRange>,
    pub(crate) symbol: Option<String>,
    pub(crate) token_estimate: u64,
    pub(crate) safe_fingerprint: String,
    pub(crate) reasons: Vec<ContextReasonCode>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextEvidenceManifestPage {
    pub(crate) items: Vec<ContextEvidenceManifest>,
    pub(crate) next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ContextSelectionError {
    ProtectedOverflow { required: u64, available: u64 },
}

pub(crate) fn select_context(
    candidates: Vec<ContextCandidate>,
    budget: &ContextBudget,
) -> Result<ContextSelection, ContextSelectionError> {
    let (mut candidates, duplicate_tokens_saved) = merge_candidates(candidates);
    candidates.retain(ContextCandidate::is_valid);
    candidates.sort_by_key(|candidate| {
        (
            Reverse(candidate.protected),
            Reverse(candidate.score()),
            candidate.id.clone(),
        )
    });

    let available = budget.evidence_budget();
    let protected = candidates
        .iter()
        .filter(|candidate| candidate.protected)
        .fold(0_u64, |total, candidate| {
            total.saturating_add(candidate.token_estimate)
        });
    if protected > available {
        return Err(ContextSelectionError::ProtectedOverflow {
            required: protected,
            available,
        });
    }

    let mut occupied = 0_u64;
    let mut source_occupied = BTreeMap::<ContextSourceKind, u64>::new();
    let mut selected = Vec::new();
    let mut rejected = Vec::new();
    for candidate in candidates {
        let source_total = source_occupied
            .get(&candidate.source_kind)
            .copied()
            .unwrap_or(0);
        let within_source = candidate.protected
            || source_total.saturating_add(candidate.token_estimate)
                <= budget.source_limit(candidate.source_kind);
        if within_source && occupied.saturating_add(candidate.token_estimate) <= available {
            occupied = occupied.saturating_add(candidate.token_estimate);
            source_occupied.insert(
                candidate.source_kind,
                source_total.saturating_add(candidate.token_estimate),
            );
            let score = candidate.score();
            selected.push(ContextEvidence {
                reasons: reasons_for(&candidate),
                candidate,
                score,
            });
        } else {
            rejected.push((candidate.id, ContextReasonCode::BudgetRejected));
        }
    }

    debug_assert!(occupied <= available);

    Ok(ContextSelection {
        policy_version: CONTEXT_ENGINE_POLICY_VERSION,
        selected,
        rejected,
        occupied_tokens: occupied,
        duplicate_tokens_saved,
    })
}

fn merge_candidates(candidates: Vec<ContextCandidate>) -> (Vec<ContextCandidate>, u64) {
    let mut merged: BTreeMap<(String, String), ContextCandidate> = BTreeMap::new();
    let mut saved = 0_u64;
    for mut candidate in candidates {
        let key = (candidate.fingerprint.clone(), candidate.source_ref.clone());
        if let Some(existing) = merged.get_mut(&key) {
            saved = saved.saturating_add(candidate.token_estimate.min(existing.token_estimate));
            existing.provenance.insert(candidate.source_kind);
            existing.provenance.append(&mut candidate.provenance);
            existing.protected |= candidate.protected;
            existing.range = existing
                .range
                .zip(candidate.range)
                .filter(|(left, right)| left.overlaps(*right))
                .map(|(left, right)| left.merge(right))
                .or(existing.range);
            continue;
        }
        candidate.provenance.insert(candidate.source_kind);
        merged.insert(key, candidate);
    }
    (merged.into_values().collect(), saved)
}

fn reasons_for(candidate: &ContextCandidate) -> Vec<ContextReasonCode> {
    let mut reasons = Vec::new();
    if candidate.protected || candidate.signals.explicitness > 0 {
        reasons.push(ContextReasonCode::Explicit);
    }
    if candidate.signals.semantic_relevance > 0 {
        reasons.push(ContextReasonCode::SemanticMatch);
    }
    if candidate.signals.symbol_relation > 0 {
        reasons.push(ContextReasonCode::SymbolRelation);
    }
    if candidate.signals.path_proximity > 0 {
        reasons.push(ContextReasonCode::PathProximity);
    }
    if candidate.signals.freshness > 0 {
        reasons.push(ContextReasonCode::Fresh);
    }
    if candidate.signals.authority > 0 {
        reasons.push(ContextReasonCode::Authoritative);
    }
    if candidate.provenance.len() > 1 {
        reasons.push(ContextReasonCode::DuplicateMerged);
    }
    reasons.push(ContextReasonCode::BudgetSelected);
    reasons
}
