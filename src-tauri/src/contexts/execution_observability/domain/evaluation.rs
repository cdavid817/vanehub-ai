use serde::{Deserialize, Serialize};

pub(crate) const EVALUATION_SCHEMA_VERSION: u16 = 1;
pub(crate) const EVALUATION_RANKING_VERSION: &str = "deterministic-v2";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvaluationCategory {
    Bugfix,
    Feature,
    Refactor,
    Tests,
    CodeReview,
    ToolUse,
    Context,
    Planning,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct EvaluationAcceptance {
    pub(crate) verifier_profiles: Vec<String>,
    pub(crate) expected_files: Vec<String>,
    pub(crate) forbidden_patterns: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct EvaluationMetricPolicy {
    pub(crate) collect_context: bool,
    pub(crate) collect_tokens: bool,
    pub(crate) collect_tools: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct EvaluationManifest {
    pub(crate) schema_version: u16,
    pub(crate) id: String,
    pub(crate) version: u32,
    pub(crate) category: EvaluationCategory,
    pub(crate) fixture: String,
    pub(crate) prompt: String,
    pub(crate) timeout_seconds: u32,
    pub(crate) acceptance: EvaluationAcceptance,
    pub(crate) metrics: EvaluationMetricPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct EvaluationAgentSnapshot {
    pub(crate) agent_id: String,
    pub(crate) provider_id: String,
    pub(crate) model_id: Option<String>,
    pub(crate) interaction_mode: String,
    pub(crate) configuration_fingerprint: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvaluationOutcome {
    Queued,
    Running,
    Succeeded,
    TaskFailed,
    AgentFailed,
    TimedOut,
    Stuck,
    Cancelled,
    BenchmarkError,
}

/// Every dispatch failure whose text is safe to show a user verbatim.
///
/// Exact strings, matched exactly. A substring allowlist is how a redaction boundary leaks: widen
/// it by a marker like `supports` and the next error that happens to contain the marker carries a
/// path or a token out with it. These are literals this codebase writes about its own
/// preconditions, so they cannot pick anything up from the runtime -- and the fixed set is what
/// makes recording a reason at all worth doing, since the two an ordinary host actually hits are
/// both here.
pub(crate) const DISPATCH_AGENT_UNAVAILABLE: &str =
    "evaluation Agent is not installed and available";
pub(crate) const DISPATCH_AGENT_UNSUPPORTED: &str =
    "evaluation supports OnePiece or an available managed CLI Agent";
pub(crate) const DISPATCH_TERMINAL_DISCONNECTED: &str =
    "evaluation Agent terminal channel disconnected";
pub(crate) const SAFE_DISPATCH_REASONS: [&str; 3] = [
    DISPATCH_AGENT_UNAVAILABLE,
    DISPATCH_AGENT_UNSUPPORTED,
    DISPATCH_TERMINAL_DISCONNECTED,
];

/// Reduces a dispatch failure to something safe to persist, export, and render.
pub(crate) fn safe_dispatch_diagnostic(error: &str) -> String {
    if SAFE_DISPATCH_REASONS.contains(&error) {
        error.to_string()
    } else {
        UNSAFE_DIAGNOSTIC.to_string()
    }
}

const UNSAFE_DIAGNOSTIC: &str =
    "evaluation operation failed; inspect unified logs for redacted diagnostics";

/// The only text an evaluation failure may say out loud.
///
/// Execution errors quote whatever the runtime handed up -- absolute paths, provider payloads, a
/// database file name -- and every place that text can reach (a persisted check summary, an export,
/// the detail pane) is a place it must already be safe. Anything not recognisably one of the safe
/// shapes collapses to a sentence that points at the redacted diagnostics instead.
pub(crate) fn safe_evaluation_error(error: String) -> String {
    const SAFE: [&str; 6] = [
        "not found",
        "requires",
        "unavailable",
        "unsupported",
        "invalid",
        "unknown",
    ];
    if error.len() <= 240
        && SAFE
            .iter()
            .any(|marker| error.to_ascii_lowercase().contains(marker))
    {
        error
    } else {
        UNSAFE_DIAGNOSTIC.to_string()
    }
}

impl EvaluationOutcome {
    /// An attempt that has reached a verdict. Anything else is still in flight, which is what
    /// separates "cancel this" from "overwrite the verdict this attempt already earned".
    pub(crate) fn is_terminal(&self) -> bool {
        !matches!(self, Self::Queued | Self::Running)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MetricQuality {
    Reported,
    Estimated,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct EvaluationMetric {
    pub(crate) name: String,
    pub(crate) value: Option<f64>,
    pub(crate) unit: String,
    pub(crate) quality: MetricQuality,
    pub(crate) source: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct EvaluationCheck {
    pub(crate) check_id: String,
    pub(crate) passed: bool,
    pub(crate) summary: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct EvaluationJudgeAssessment {
    pub(crate) model_id: String,
    pub(crate) rubric_version: String,
    pub(crate) prompt_version: String,
    pub(crate) seed: Option<u64>,
    pub(crate) temperature: Option<f64>,
    pub(crate) passed: bool,
    pub(crate) confidence: Option<f64>,
    pub(crate) evidence_ids: Vec<String>,
    pub(crate) notes: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct EvaluationAttempt {
    pub(crate) id: String,
    pub(crate) arena_id: String,
    pub(crate) canonical_run_id: String,
    pub(crate) task_id: String,
    pub(crate) task_version: u32,
    pub(crate) agent: EvaluationAgentSnapshot,
    pub(crate) outcome: EvaluationOutcome,
    pub(crate) checks: Vec<EvaluationCheck>,
    pub(crate) judge: Option<EvaluationJudgeAssessment>,
    pub(crate) metrics: Vec<EvaluationMetric>,
    pub(crate) context_evidence_manifest_id: Option<String>,
    pub(crate) artifact_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct EvaluationArena {
    pub(crate) id: String,
    pub(crate) operation_id: String,
    pub(crate) task_id: String,
    pub(crate) task_version: u32,
    pub(crate) ranking_version: String,
    pub(crate) attempts: Vec<EvaluationAttempt>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct EvaluationExport {
    pub(crate) schema_version: u16,
    pub(crate) arena: EvaluationArena,
}
