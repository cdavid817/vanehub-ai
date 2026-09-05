use serde::{Deserialize, Serialize};

use super::{
    EvolutionActorProvenance, EvolutionCheckpointStatus, EvolutionRunStatus, EvolutionStageKind,
    EvolutionStageStatus, EvolutionTriggerCountersV1,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvolutionRunBudgetV1 {
    pub(crate) wall_time_ms: u64,
    pub(crate) evidence_items: u32,
    pub(crate) seed_groups: u16,
    pub(crate) assessments: u16,
    pub(crate) model_calls: u16,
    pub(crate) notifications: u16,
    pub(crate) automatic_mutations: u8,
}

impl EvolutionRunBudgetV1 {
    pub(crate) fn automatic_v1() -> Self {
        Self {
            wall_time_ms: 120_000,
            evidence_items: 1_000,
            seed_groups: 100,
            assessments: 25,
            model_calls: 10,
            notifications: 20,
            automatic_mutations: 1,
        }
    }

    pub(crate) fn manual_v1() -> Self {
        Self {
            wall_time_ms: 300_000,
            evidence_items: 5_000,
            seed_groups: 500,
            assessments: 100,
            model_calls: 25,
            notifications: 50,
            automatic_mutations: 1,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvolutionRunUsageV1 {
    pub(crate) elapsed_ms: u64,
    pub(crate) evidence_items: u32,
    pub(crate) seed_groups: u16,
    pub(crate) assessments: u16,
    pub(crate) model_calls: u16,
    pub(crate) notifications: u16,
    pub(crate) automatic_mutations: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvolutionRunRequestV1 {
    pub(crate) schema_version: u16,
    pub(crate) request_id: String,
    pub(crate) workspace_id: String,
    pub(crate) actor: EvolutionActorProvenance,
    pub(crate) trigger_counters: EvolutionTriggerCountersV1,
    pub(crate) trigger_receipt_ids: Vec<String>,
    pub(crate) follow_up: bool,
    pub(crate) not_before_ms: i64,
    pub(crate) created_at_ms: i64,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvolutionRunV1 {
    pub(crate) schema_version: u16,
    pub(crate) run_id: String,
    pub(crate) request_id: String,
    pub(crate) workspace_id: String,
    pub(crate) status: EvolutionRunStatus,
    pub(crate) current_stage: Option<EvolutionStageKind>,
    pub(crate) policy_witness_hash: String,
    pub(crate) budget: EvolutionRunBudgetV1,
    pub(crate) usage: EvolutionRunUsageV1,
    pub(crate) cancel_requested_at_ms: Option<i64>,
    pub(crate) lease_owner: Option<String>,
    pub(crate) lease_expires_at_ms: Option<i64>,
    pub(crate) safe_failure_code: Option<String>,
    pub(crate) revision: u64,
    pub(crate) created_at_ms: i64,
    pub(crate) updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvolutionRunStageV1 {
    pub(crate) stage_id: String,
    pub(crate) run_id: String,
    pub(crate) stage: EvolutionStageKind,
    pub(crate) attempt: u16,
    pub(crate) status: EvolutionStageStatus,
    pub(crate) input_witness_hash: String,
    pub(crate) output_witness_hash: Option<String>,
    pub(crate) safe_failure_code: Option<String>,
    pub(crate) started_at_ms: Option<i64>,
    pub(crate) completed_at_ms: Option<i64>,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvolutionRunItemV1 {
    pub(crate) item_id: String,
    pub(crate) run_id: String,
    pub(crate) stage: EvolutionStageKind,
    pub(crate) subsystem_idempotency_key: String,
    pub(crate) source_id: String,
    pub(crate) source_revision: u64,
    pub(crate) committed_receipt_id: Option<String>,
    pub(crate) safe_failure_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvolutionCheckpointV1 {
    pub(crate) checkpoint_id: String,
    pub(crate) run_id: String,
    pub(crate) stage: EvolutionStageKind,
    pub(crate) status: EvolutionCheckpointStatus,
    pub(crate) cursor_record_id: Option<String>,
    pub(crate) cursor_record_revision: Option<u64>,
    pub(crate) usage: EvolutionRunUsageV1,
    pub(crate) continuation_not_before_ms: Option<i64>,
    pub(crate) committed_at_ms: i64,
}
