use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartEvaluationInput {
    pub(crate) task_id: String,
    pub(crate) task_version: u32,
    pub(crate) agent_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvaluationTask {
    pub(crate) id: String,
    pub(crate) version: u32,
    pub(crate) category: String,
    pub(crate) prompt: String,
    pub(crate) timeout_seconds: u32,
    pub(crate) verifier_profiles: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvaluationAgentSnapshot {
    pub(crate) agent_id: String,
    pub(crate) provider_id: String,
    pub(crate) model_id: Option<String>,
    pub(crate) interaction_mode: String,
    pub(crate) configuration_fingerprint: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvaluationCheck {
    pub(crate) check_id: String,
    pub(crate) passed: bool,
    pub(crate) summary: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvaluationMetric {
    pub(crate) name: String,
    pub(crate) value: Option<f64>,
    pub(crate) unit: String,
    pub(crate) quality: String,
    pub(crate) source: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvaluationAttempt {
    pub(crate) id: String,
    pub(crate) arena_id: String,
    pub(crate) canonical_run_id: String,
    pub(crate) task_id: String,
    pub(crate) task_version: u32,
    pub(crate) agent: EvaluationAgentSnapshot,
    pub(crate) outcome: String,
    pub(crate) checks: Vec<EvaluationCheck>,
    pub(crate) judge: Option<serde_json::Value>,
    pub(crate) metrics: Vec<EvaluationMetric>,
    pub(crate) context_evidence_manifest_id: Option<String>,
    pub(crate) artifact_ids: Vec<String>,
    pub(crate) timeline: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvaluationArena {
    pub(crate) id: String,
    pub(crate) operation_id: String,
    pub(crate) task_id: String,
    pub(crate) task_version: u32,
    pub(crate) ranking_version: String,
    pub(crate) attempts: Vec<EvaluationAttempt>,
}

/// 18.6: the frontend-facing shape for `list_evaluation_arenas`, cursor-shaped like
/// `MissionControlPage`/`CursorPage` rather than raw offset/limit -- see that command's own doc
/// comment for why, given the repository underneath is genuinely OFFSET/LIMIT, not a keyset cursor.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvaluationArenaPage {
    pub(crate) items: Vec<EvaluationArena>,
    pub(crate) next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvaluationExport {
    pub(crate) schema_version: u16,
    pub(crate) arena: EvaluationArena,
}
