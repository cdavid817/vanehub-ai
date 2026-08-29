use super::domain::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorQueueQuery {
    pub(crate) workspace_id: String,
    pub(crate) skill_id: Option<String>,
    #[serde(default)]
    pub(crate) states: Vec<String>,
    #[serde(default)]
    pub(crate) routes: Vec<String>,
    #[serde(default)]
    pub(crate) risks: Vec<String>,
    pub(crate) draft_ready: Option<bool>,
    pub(crate) stale: Option<bool>,
    pub(crate) notification_pending: Option<bool>,
    pub(crate) updated_before_ms: Option<i64>,
    pub(crate) limit: Option<usize>,
    pub(crate) cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorAuditQuery {
    pub(crate) candidate_id: String,
    pub(crate) cursor: Option<String>,
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorDraftInput {
    pub(crate) schema_version: u16,
    pub(crate) candidate_id: String,
    pub(crate) expected_candidate_revision: u64,
    pub(crate) target_skill_id: Option<String>,
    pub(crate) target_revision: Option<String>,
    pub(crate) overlay_scope: Option<String>,
    pub(crate) mutation: CuratorDraftMutationInput,
    pub(crate) rationale: String,
    pub(crate) expected_effective_change: String,
    pub(crate) idempotency_key: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorPreviewInput {
    pub(crate) candidate_id: String,
    pub(crate) expected_candidate_revision: u64,
    pub(crate) expected_draft_revision: u64,
    pub(crate) expected_assessment_id: String,
    pub(crate) idempotency_key: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorApproveInput {
    pub(crate) candidate_id: String,
    pub(crate) expected_candidate_revision: u64,
    pub(crate) confirmed_preview_hash: String,
    pub(crate) confirmed_effective_diff_hash: String,
    pub(crate) idempotency_key: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorRejectInput {
    pub(crate) candidate_id: String,
    pub(crate) expected_candidate_revision: u64,
    pub(crate) idempotency_key: String,
    pub(crate) reason: CuratorRejectionReason,
    pub(crate) note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorDeferInput {
    pub(crate) candidate_id: String,
    pub(crate) expected_candidate_revision: u64,
    pub(crate) idempotency_key: String,
    pub(crate) reason: CuratorDeferReason,
    pub(crate) note: Option<String>,
    pub(crate) review_after_ms: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorResumeInput {
    pub(crate) candidate_id: String,
    pub(crate) expected_candidate_revision: u64,
    pub(crate) expected_candidate_hash: String,
    pub(crate) expected_policy_hash: String,
    pub(crate) expected_draft_revision: Option<u64>,
    pub(crate) expected_assessment_id: Option<String>,
    pub(crate) idempotency_key: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorRetryInput {
    pub(crate) candidate_id: String,
    pub(crate) expected_candidate_revision: u64,
    pub(crate) idempotency_key: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorPolicyInput {
    pub(crate) workspace_id: String,
    pub(crate) expected_revision: u64,
    pub(crate) policy: CuratorPolicyPatch,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorPolicyPatch {
    pub(crate) enqueue_routes: Vec<CuratorRoute>,
    pub(crate) require_rejection_reason: bool,
    pub(crate) require_defer_reason: bool,
    pub(crate) maximum_defer_days: u16,
    pub(crate) open_retention_days: u16,
    pub(crate) terminal_retention_days: u16,
    pub(crate) notifications_enabled: bool,
    pub(crate) digest_enabled: bool,
    pub(crate) draft_display_limit_bytes: u32,
    pub(crate) diff_display_limit_bytes: u32,
}

impl From<CuratorPolicyPatch> for CuratorPolicyUpdateV1 {
    fn from(value: CuratorPolicyPatch) -> Self {
        Self {
            schema_version: CURATOR_SCHEMA_VERSION_V1,
            enqueue_routes: value.enqueue_routes,
            require_rejection_reason: value.require_rejection_reason,
            require_defer_reason: value.require_defer_reason,
            maximum_defer_days: value.maximum_defer_days,
            open_retention_days: value.open_retention_days,
            terminal_retention_days: value.terminal_retention_days,
            notifications_enabled: value.notifications_enabled,
            digest_enabled: value.digest_enabled,
            draft_display_limit_bytes: value.draft_display_limit_bytes,
            diff_display_limit_bytes: value.diff_display_limit_bytes,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CuratorSafeState {
    pub(crate) candidate_id: String,
    pub(crate) revision: u64,
    pub(crate) state: String,
    pub(crate) witness_hash: String,
    pub(crate) policy_witness_hash: String,
    pub(crate) current_preview_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CuratorApiError {
    pub(crate) code: &'static str,
    pub(crate) message: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) current: Option<Box<CuratorSafeState>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reason_code: Option<String>,
}

impl CuratorApiError {
    pub(crate) fn new(code: &'static str) -> Self {
        Self {
            code,
            message: code,
            current: None,
            reason_code: None,
        }
    }

    pub(crate) fn reason(code: &'static str, reason_code: String) -> Self {
        Self {
            reason_code: Some(reason_code),
            ..Self::new(code)
        }
    }
}

pub(crate) type CuratorApiResult = Result<Value, CuratorApiError>;
