use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SystemActivitySession {
    pub(crate) schema_version: u16,
    pub(crate) session_id: String,
    pub(crate) activity_kind: ActivityKind,
    pub(crate) scope_kind: ActivityScopeKind,
    pub(crate) canonical_scope_id: String,
    pub(crate) safe_display_identity: Option<String>,
    pub(crate) active_generation_id: String,
    pub(crate) last_sequence: u64,
    pub(crate) unread_count: u64,
    pub(crate) attention: ActivityAttentionKind,
    pub(crate) preference_revision: u64,
    pub(crate) created_at_ms: i64,
    pub(crate) first_activity_at_ms: i64,
    pub(crate) last_activity_at_ms: i64,
    pub(crate) last_projected_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ActivitySourceReceipt {
    pub(crate) source_domain: String,
    pub(crate) source_id: String,
    pub(crate) source_revision: String,
    pub(crate) event_code: ActivityEventCode,
    pub(crate) projection_version: u16,
    pub(crate) event_id: String,
    pub(crate) source_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ActivityTargetReceipt {
    pub(crate) event_id: String,
    pub(crate) target_kind: ActivityTargetKind,
    pub(crate) target_scope: String,
    pub(crate) status: ActivityDeliveryStatus,
    pub(crate) delivered_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ActivityDomainCursor {
    pub(crate) source_domain: EvolutionSourceDomain,
    pub(crate) opaque_cursor: Option<OpaqueDomainCursor>,
    pub(crate) last_sequence: u64,
    pub(crate) last_source_hash: Option<String>,
    pub(crate) retention_floor: Option<OpaqueDomainCursor>,
    pub(crate) pending_count: u64,
    pub(crate) oldest_pending_at_ms: Option<i64>,
    pub(crate) gap: Option<ActivityGapCode>,
    pub(crate) failure_code: Option<ActivityProjectionFailureCode>,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivityProjectionLease {
    pub(crate) owner_id: String,
    pub(crate) expires_at_ms: i64,
    pub(crate) heartbeat_at_ms: i64,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivityDomainCheckpoint {
    pub(crate) source_domain: EvolutionSourceDomain,
    pub(crate) opaque_cursor: OpaqueDomainCursor,
    pub(crate) last_sequence: u64,
    pub(crate) last_source_hash: String,
    pub(crate) retention_floor: Option<OpaqueDomainCursor>,
    pub(crate) pending_count: u64,
    pub(crate) oldest_pending_at_ms: Option<i64>,
    pub(crate) last_success_at_ms: i64,
    pub(crate) expected_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SystemActivityReadState {
    pub(crate) session_id: String,
    pub(crate) user_id: String,
    pub(crate) highest_read_sequence: u64,
    pub(crate) mark_unread_sequence: Option<u64>,
    pub(crate) last_seen_at_ms: i64,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvolutionActivityPreferences {
    pub(crate) scope_kind: ActivityScopeKind,
    pub(crate) canonical_scope_id: String,
    pub(crate) visible: bool,
    pub(crate) minimum_timeline_severity: ActivitySeverity,
    pub(crate) notification_threshold: ActivitySeverity,
    pub(crate) digest_cadence: ActivityDigestCadence,
    pub(crate) read_retention_days: u16,
    pub(crate) detail_retention_days: u16,
    pub(crate) export_item_limit: u32,
    pub(crate) export_size_limit_bytes: u64,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ActivityProjectionHealth {
    pub(crate) active_generation_id: String,
    pub(crate) lease_owner: Option<String>,
    pub(crate) domains: Vec<ActivityDomainCursor>,
    pub(crate) last_completed_at_ms: Option<i64>,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ActivityRebuild {
    pub(crate) rebuild_id: String,
    pub(crate) scope_kind: ActivityScopeKind,
    pub(crate) canonical_scope_id: String,
    pub(crate) shadow_generation_id: String,
    pub(crate) source_snapshot_hash: String,
    pub(crate) status: ActivityRebuildStatus,
    pub(crate) processed_items: u64,
    pub(crate) item_budget: u64,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ActivityDigest {
    pub(crate) bucket_id: String,
    pub(crate) cadence: ActivityDigestCadence,
    pub(crate) window_started_at_ms: i64,
    pub(crate) window_ends_at_ms: i64,
    pub(crate) item_count: u32,
    pub(crate) highest_severity: ActivitySeverity,
    pub(crate) delivered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ActivityExport {
    pub(crate) export_id: String,
    pub(crate) session_id: String,
    pub(crate) generation_id: String,
    pub(crate) format: ActivityExportFormat,
    pub(crate) item_count: u32,
    pub(crate) byte_count: u64,
    pub(crate) complete: bool,
    pub(crate) redaction_version: String,
    pub(crate) content_hash: String,
}
