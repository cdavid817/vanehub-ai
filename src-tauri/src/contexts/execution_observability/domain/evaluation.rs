use serde::{Deserialize, Serialize};

pub(crate) const EVALUATION_SCHEMA_VERSION: u16 = 1;
pub(crate) const EVALUATION_RANKING_VERSION: &str = "deterministic-v1";

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
