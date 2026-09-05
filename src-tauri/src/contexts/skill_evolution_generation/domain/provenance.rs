use serde::{Deserialize, Serialize};

use super::{GenerationModelOutcome, GenerationToolOutcome};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GenerationModelCallRecordV1 {
    pub(crate) model_call_id: String,
    pub(crate) stage_attempt_id: String,
    pub(crate) purpose: String,
    pub(crate) provider_protocol: Option<String>,
    pub(crate) provider_profile_id: Option<String>,
    pub(crate) model_id: Option<String>,
    pub(crate) prompt_template_version: String,
    pub(crate) response_schema_version: String,
    pub(crate) outcome: GenerationModelOutcome,
    pub(crate) input_tokens: u64,
    pub(crate) output_tokens: u64,
    pub(crate) latency_ms: u64,
    pub(crate) structured_response_hash: Option<String>,
    pub(crate) safe_failure_code: Option<String>,
    pub(crate) created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GenerationToolReceiptV1 {
    pub(crate) receipt_id: String,
    pub(crate) stage_attempt_id: String,
    pub(crate) tool_name: String,
    pub(crate) argument_hash: String,
    pub(crate) source_witness_hash: String,
    pub(crate) outcome: GenerationToolOutcome,
    pub(crate) result_hash: Option<String>,
    pub(crate) safe_failure_code: Option<String>,
    pub(crate) duration_ms: u64,
    pub(crate) created_at_ms: i64,
}
