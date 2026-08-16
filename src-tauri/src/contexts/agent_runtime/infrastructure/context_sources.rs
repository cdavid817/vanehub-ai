use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentCodeIntelligenceContext, AgentCodeIntelligencePort,
    AgentCodeIntelligenceStatus, AgentDocumentPositionInput, AgentLog, AgentLogLevel,
    AgentLoggingPort, AgentRetrievalPort, ContextCandidateSource, ContextEngineClockPort,
    ContextEngineDiagnostic, ContextEngineDiagnosticPort, ContextSourceResult,
};
use crate::contexts::agent_runtime::domain::{
    CandidateSignals, ContextCandidate, ContextRange, ContextRequest, ContextSourceKind,
    ContextSourceOutcome, EstimateQuality,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Component, Path};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub(crate) struct MonotonicContextEngineClock(Instant);

impl Default for MonotonicContextEngineClock {
    fn default() -> Self {
        Self(Instant::now())
    }
}

impl ContextEngineClockPort for MonotonicContextEngineClock {
    fn now_millis(&self) -> u64 {
        u64::try_from(self.0.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

pub(crate) struct UnifiedContextEngineDiagnostics {
    logging: Arc<dyn AgentLoggingPort>,
    clock: Arc<dyn AgentClockPort>,
}

impl UnifiedContextEngineDiagnostics {
    pub(crate) fn new(logging: Arc<dyn AgentLoggingPort>, clock: Arc<dyn AgentClockPort>) -> Self {
        Self { logging, clock }
    }
}

impl ContextEngineDiagnosticPort for UnifiedContextEngineDiagnostics {
    fn record(&self, event: ContextEngineDiagnostic) {
        let _ = self.logging.record(AgentLog {
            level: AgentLogLevel::Debug,
            category: "session.runtime.context-engine".to_string(),
            message: format!(
                "policy={} outcome={} selected={} rejected={} occupied_tokens={}",
                event.policy_version,
                event.outcome,
                event.selected_count,
                event.rejected_count,
                event.occupied_tokens
            ),
            agent_id: None,
            session_id: None,
            operation_id: None,
            run_id: None,
            trace_id: None,
            span_id: None,
            occurred_at: self.clock.now(),
        });
    }
}

pub(crate) struct ExplicitReferenceContextSource;

impl ContextCandidateSource for ExplicitReferenceContextSource {
    fn kind(&self) -> ContextSourceKind {
        ContextSourceKind::ExplicitReference
    }

    fn collect(&self, request: &ContextRequest, cancelled: &AtomicBool) -> ContextSourceResult {
        let Some(workspace) = request.workspace_ref.as_deref() else {
            return unavailable();
        };
        let root = Path::new(workspace);
        let mut candidates = Vec::new();
        for source_ref in request.explicit_refs.iter().take(16) {
            if cancelled.load(Ordering::Relaxed) {
                return cancelled_result();
            }
            if !safe_relative(source_ref) {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(root.join(source_ref)) else {
                continue;
            };
            candidates.push(candidate(
                format!("explicit:{}", fingerprint(source_ref)),
                ContextSourceKind::ExplicitReference,
                source_ref.clone(),
                content,
                None,
                None,
                true,
                CandidateSignals {
                    explicitness: 100,
                    semantic_relevance: 80,
                    symbol_relation: 0,
                    path_proximity: 100,
                    freshness: 50,
                    authority: 70,
                },
            ));
        }
        ready(candidates)
    }
}

pub(crate) struct RetrievalContextSource {
    retrieval: Arc<dyn AgentRetrievalPort>,
    kind: ContextSourceKind,
}

pub(crate) struct CodeIntelligenceContextSource {
    code_intelligence: Arc<dyn AgentCodeIntelligencePort>,
    kind: ContextSourceKind,
}

impl CodeIntelligenceContextSource {
    pub(crate) fn definition(code_intelligence: Arc<dyn AgentCodeIntelligencePort>) -> Self {
        Self {
            code_intelligence,
            kind: ContextSourceKind::LspDefinition,
        }
    }
    pub(crate) fn references(code_intelligence: Arc<dyn AgentCodeIntelligencePort>) -> Self {
        Self {
            code_intelligence,
            kind: ContextSourceKind::LspReference,
        }
    }
}

impl ContextCandidateSource for CodeIntelligenceContextSource {
    fn kind(&self) -> ContextSourceKind {
        self.kind
    }

    fn collect(&self, request: &ContextRequest, cancelled: &AtomicBool) -> ContextSourceResult {
        let Some(workspace) = request.workspace_ref.as_deref() else {
            return unavailable();
        };
        let Some(reference) = request
            .explicit_refs
            .iter()
            .find(|value| safe_relative(value))
        else {
            return unavailable();
        };
        let context = AgentCodeIntelligenceContext::from_session_workspace(workspace);
        if !self.code_intelligence.is_available(&context) {
            return unavailable();
        }
        if cancelled.load(Ordering::Relaxed) {
            return cancelled_result();
        }
        let cancellation = Arc::new(AtomicBool::new(false));
        let input = AgentDocumentPositionInput {
            relative_path: reference.clone(),
            line: 0,
            column: 0,
        };
        let outcome = if self.kind == ContextSourceKind::LspDefinition {
            self.code_intelligence
                .find_definition(&context, &input, cancellation)
        } else {
            self.code_intelligence
                .find_references(&context, &input, cancellation)
        };
        let status = code_intelligence_outcome(outcome.metadata.status);
        let candidates = outcome
            .value
            .unwrap_or_default()
            .into_iter()
            .take(24)
            .filter(|location| safe_relative(&location.file))
            .filter_map(|location| {
                let content = location.preview?;
                Some(candidate(
                    format!(
                        "lsp:{}:{}",
                        fingerprint(&location.file),
                        location.range.start_line
                    ),
                    self.kind,
                    location.file,
                    content,
                    ContextRange::new(
                        location.range.start_line.saturating_add(1),
                        location.range.end_line.saturating_add(1),
                    ),
                    None,
                    false,
                    CandidateSignals {
                        explicitness: 0,
                        semantic_relevance: 75,
                        symbol_relation: 100,
                        path_proximity: 60,
                        freshness: 70,
                        authority: 70,
                    },
                ))
            })
            .collect();
        ContextSourceResult {
            outcome: status,
            candidates,
        }
    }
}

impl RetrievalContextSource {
    pub(crate) fn workspace(retrieval: Arc<dyn AgentRetrievalPort>) -> Self {
        Self {
            retrieval,
            kind: ContextSourceKind::Retrieval,
        }
    }

    pub(crate) fn memory(retrieval: Arc<dyn AgentRetrievalPort>) -> Self {
        Self {
            retrieval,
            kind: ContextSourceKind::Memory,
        }
    }
}

impl ContextCandidateSource for RetrievalContextSource {
    fn kind(&self) -> ContextSourceKind {
        self.kind
    }

    fn collect(&self, request: &ContextRequest, cancelled: &AtomicBool) -> ContextSourceResult {
        if cancelled.load(Ordering::Relaxed) {
            return cancelled_result();
        }
        if self.kind == ContextSourceKind::Memory {
            return match self.retrieval.search(&request.task, 8) {
                Ok(outcome) => ContextSourceResult {
                    outcome: degradation(outcome.degraded.as_deref()),
                    candidates: outcome
                        .hits
                        .into_iter()
                        .enumerate()
                        .map(|(index, hit)| {
                            let hash = fingerprint(&hit.content);
                            candidate(
                                format!("memory:{index}:{hash}"),
                                self.kind,
                                format!("memory/{hash}"),
                                hit.content,
                                None,
                                None,
                                false,
                                CandidateSignals {
                                    explicitness: 0,
                                    semantic_relevance: 70,
                                    symbol_relation: 0,
                                    path_proximity: 0,
                                    freshness: 60,
                                    authority: 30,
                                },
                            )
                        })
                        .collect(),
                },
                Err(_) => failed(),
            };
        }
        let Some(workspace) = request.workspace_ref.as_deref() else {
            return unavailable();
        };
        let Some(code) = self.retrieval.code_retrieval() else {
            return unavailable();
        };
        if !code.is_available(workspace) {
            return unavailable();
        }
        match code.search_code(workspace, &request.task, 24) {
            Ok(outcome) => ContextSourceResult {
                outcome: degradation(outcome.degraded.as_deref()),
                candidates: outcome
                    .hits
                    .into_iter()
                    .filter(|hit| safe_relative(&hit.file_path))
                    .map(|hit| {
                        let range = ContextRange::new(hit.start_line, hit.end_line);
                        let source_kind = if hit.file_path.contains("test") {
                            ContextSourceKind::Test
                        } else if hit.matched_via.contains("tree") {
                            ContextSourceKind::TreeSitter
                        } else {
                            self.kind
                        };
                        candidate(
                            format!(
                                "code:{}:{}:{}",
                                fingerprint(&hit.file_path),
                                hit.start_line,
                                hit.end_line
                            ),
                            source_kind,
                            hit.file_path,
                            hit.snippet,
                            range,
                            hit.symbol_name,
                            false,
                            CandidateSignals {
                                explicitness: 0,
                                semantic_relevance: 80,
                                symbol_relation: 70,
                                path_proximity: 50,
                                freshness: 50,
                                authority: 60,
                            },
                        )
                    })
                    .collect(),
            },
            Err(_) => failed(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn candidate(
    id: String,
    source_kind: ContextSourceKind,
    source_ref: String,
    content: String,
    range: Option<ContextRange>,
    symbol: Option<String>,
    protected: bool,
    signals: CandidateSignals,
) -> ContextCandidate {
    let fingerprint = fingerprint(&content);
    ContextCandidate {
        id,
        source_kind,
        source_ref,
        token_estimate: ((content.chars().count() as u64).saturating_add(3) / 4).max(1),
        estimate_quality: EstimateQuality::CharactersOnly,
        content,
        range,
        symbol,
        signals,
        redundancy_group: Some(fingerprint.clone()),
        protected,
        fingerprint,
        provenance: BTreeSet::from([source_kind]),
    }
}

fn fingerprint(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let encoded = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256:{encoded}")
}

fn safe_relative(value: &str) -> bool {
    let path = Path::new(value);
    !path.is_absolute()
        && !value.trim().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn degradation(value: Option<&str>) -> ContextSourceOutcome {
    value.map_or(ContextSourceOutcome::Ready, |_| {
        ContextSourceOutcome::Warming
    })
}
fn code_intelligence_outcome(status: AgentCodeIntelligenceStatus) -> ContextSourceOutcome {
    match status {
        AgentCodeIntelligenceStatus::Ready => ContextSourceOutcome::Ready,
        AgentCodeIntelligenceStatus::Warming => ContextSourceOutcome::Warming,
        AgentCodeIntelligenceStatus::Timeout => ContextSourceOutcome::TimedOut,
        AgentCodeIntelligenceStatus::Unavailable => ContextSourceOutcome::Unavailable,
        AgentCodeIntelligenceStatus::Failed => ContextSourceOutcome::Failed,
    }
}
fn ready(candidates: Vec<ContextCandidate>) -> ContextSourceResult {
    ContextSourceResult {
        outcome: ContextSourceOutcome::Ready,
        candidates,
    }
}
fn unavailable() -> ContextSourceResult {
    ContextSourceResult {
        outcome: ContextSourceOutcome::Unavailable,
        candidates: Vec::new(),
    }
}
fn failed() -> ContextSourceResult {
    ContextSourceResult {
        outcome: ContextSourceOutcome::Failed,
        candidates: Vec::new(),
    }
}
fn cancelled_result() -> ContextSourceResult {
    ContextSourceResult {
        outcome: ContextSourceOutcome::Cancelled,
        candidates: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::application::{
        AgentRuntimeApplicationError, ContextEngineDiagnostic,
    };
    use std::sync::Mutex;

    #[derive(Default)]
    struct Logs(Mutex<Vec<AgentLog>>);
    impl AgentLoggingPort for Logs {
        fn record(&self, log: AgentLog) -> Result<(), AgentRuntimeApplicationError> {
            self.0.lock().expect("logs").push(log);
            Ok(())
        }
    }
    struct Clock;
    impl AgentClockPort for Clock {
        fn now(&self) -> String {
            "2026-01-01T00:00:00Z".to_string()
        }
    }

    #[test]
    fn explicit_source_rejects_escape_and_reads_only_confined_text() {
        let directory = tempfile::tempdir().expect("tempdir");
        std::fs::write(directory.path().join("safe.rs"), "fn safe() {}").expect("fixture");
        let request = ContextRequest {
            session_id: "s".to_string(),
            turn_id: "t".to_string(),
            generation_id: "g".to_string(),
            task: "secret prompt".to_string(),
            workspace_ref: Some(directory.path().to_string_lossy().to_string()),
            explicit_refs: vec![
                "safe.rs".to_string(),
                "../secret.env".to_string(),
                "/etc/passwd".to_string(),
            ],
            model_capacity: Some(100),
        };
        let result = ExplicitReferenceContextSource.collect(&request, &AtomicBool::new(false));
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.candidates[0].source_ref, "safe.rs");
    }

    #[test]
    fn unified_diagnostic_excludes_prompt_source_memory_credentials_and_payloads() {
        let logs = Arc::new(Logs::default());
        let diagnostic = UnifiedContextEngineDiagnostics::new(logs.clone(), Arc::new(Clock));
        diagnostic.record(ContextEngineDiagnostic {
            policy_version: "context-engine-v1",
            selected_count: 2,
            rejected_count: 1,
            occupied_tokens: 100,
            outcome: "ready",
        });
        let text = &logs.0.lock().expect("logs")[0].message;
        assert!(text.contains("selected=2"));
        for secret in [
            "secret prompt",
            "fn safe",
            "memory body",
            "Bearer",
            "tool payload",
            "raw frame",
        ] {
            assert!(!text.contains(secret));
        }
    }
}
