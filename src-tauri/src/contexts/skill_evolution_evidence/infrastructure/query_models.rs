use std::collections::BTreeMap;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvidenceQueryScope {
    pub(crate) workspace: Option<String>,
    pub(crate) skill_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvidencePageRequest {
    pub(crate) scope: EvidenceQueryScope,
    pub(crate) limit: usize,
    pub(crate) cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidencePipelineSummary {
    pub(crate) collection_enabled: bool,
    pub(crate) status: String,
    pub(crate) queue_depth: usize,
    pub(crate) failure_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceSignalSummary {
    pub(crate) signal_id: String,
    pub(crate) source_kind: String,
    pub(crate) category: String,
    pub(crate) polarity: String,
    pub(crate) severity: String,
    pub(crate) attribution: String,
    pub(crate) attribution_rationale: String,
    pub(crate) source_fidelity: String,
    pub(crate) source_agent_id: Option<String>,
    pub(crate) extractor_id: String,
    pub(crate) extractor_version: u16,
    pub(crate) safe_summary: String,
    pub(crate) occurred_at: String,
    pub(crate) sanitizer_version: u16,
    pub(crate) association_truncated_count: usize,
    pub(crate) source_link_truncated_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceSeedSummary {
    pub(crate) seed_id: String,
    pub(crate) category: String,
    pub(crate) readiness: String,
    pub(crate) readiness_reason: String,
    pub(crate) safe_summary: String,
    pub(crate) signal_count: usize,
    pub(crate) truncated_signal_count: usize,
    pub(crate) independent_run_count: usize,
    pub(crate) has_recovery: bool,
    pub(crate) first_occurred_at: String,
    pub(crate) last_occurred_at: String,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceOverview {
    pub(crate) signal_count: usize,
    pub(crate) seed_count: usize,
    pub(crate) first_occurred_at: Option<String>,
    pub(crate) last_occurred_at: Option<String>,
    pub(crate) distributions: BTreeMap<String, BTreeMap<String, usize>>,
    pub(crate) signals: Vec<EvidenceSignalSummary>,
    pub(crate) seeds: Vec<EvidenceSeedSummary>,
    pub(crate) next_cursor: Option<String>,
    pub(crate) pipeline: EvidencePipelineSummary,
    pub(crate) retention_days: i64,
    pub(crate) signal_quota: usize,
    pub(crate) seed_quota: usize,
    pub(crate) byte_quota: usize,
    pub(crate) dropped_count: usize,
    pub(crate) expired_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceSeedLineage {
    pub(crate) seed: EvidenceSeedSummary,
    pub(crate) signals: Vec<EvidenceSignalSummary>,
}
