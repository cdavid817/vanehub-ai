use super::{GenerationJobStatus, GenerationStageKind, GenerationStageStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GenerationJobV1 {
    pub(crate) schema_version: u16,
    pub(crate) job_id: String,
    pub(crate) request_id: String,
    pub(crate) workspace_id: Option<String>,
    pub(crate) status: GenerationJobStatus,
    pub(crate) current_stage: Option<GenerationStageKind>,
    pub(crate) input_witness_hash: String,
    pub(crate) current_attempt: u32,
    pub(crate) budget: GenerationBudgetV1,
    pub(crate) usage: GenerationUsageV1,
    pub(crate) safe_failure_code: Option<String>,
    pub(crate) supersedes_job_id: Option<String>,
    pub(crate) created_at_ms: i64,
    pub(crate) updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GenerationStageAttemptV1 {
    pub(crate) attempt_id: String,
    pub(crate) job_id: String,
    pub(crate) stage: GenerationStageKind,
    pub(crate) attempt: u16,
    pub(crate) status: GenerationStageStatus,
    pub(crate) input_hash: String,
    pub(crate) output_hash: Option<String>,
    pub(crate) usage: GenerationUsageV1,
    pub(crate) safe_failure_code: Option<String>,
    pub(crate) started_at_ms: i64,
    pub(crate) completed_at_ms: Option<i64>,
    pub(crate) superseded_by_attempt_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GenerationBudgetV1 {
    pub(crate) wall_time_ms: u64,
    pub(crate) model_calls: u16,
    pub(crate) tool_calls: u16,
    pub(crate) input_tokens: u64,
    pub(crate) output_tokens: u64,
    pub(crate) validation_repairs: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GenerationUsageV1 {
    pub(crate) elapsed_ms: u64,
    pub(crate) model_calls: u16,
    pub(crate) tool_calls: u16,
    pub(crate) input_tokens: u64,
    pub(crate) output_tokens: u64,
    pub(crate) validation_repairs: u8,
}
