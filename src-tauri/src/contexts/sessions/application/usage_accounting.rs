#![allow(dead_code)]

use crate::contexts::sessions::domain::{
    AccountingUnit, MeasurementKind, MeasurementQuality, TokenDimensions, TokenOverlap,
    UsageInteractionKind, UsagePurpose, UsageStatus,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewModelInvocation {
    pub(crate) id: String,
    pub(crate) generation_id: Option<String>,
    pub(crate) run_id: Option<String>,
    pub(crate) operation_id: Option<String>,
    pub(crate) session_id: String,
    pub(crate) message_id: Option<String>,
    pub(crate) agent_id: String,
    pub(crate) provider_id: Option<String>,
    pub(crate) profile_id: Option<String>,
    pub(crate) endpoint_id: Option<String>,
    pub(crate) model_id: Option<String>,
    pub(crate) interaction_kind: UsageInteractionKind,
    pub(crate) purpose: UsagePurpose,
    pub(crate) request_sequence: u32,
    pub(crate) attempt: u32,
    pub(crate) started_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelInvocationRecord {
    pub(crate) invocation: NewModelInvocation,
    pub(crate) status: UsageStatus,
    pub(crate) completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewUsageObservation {
    pub(crate) id: String,
    pub(crate) invocation_id: String,
    pub(crate) quality: MeasurementQuality,
    pub(crate) unit: AccountingUnit,
    pub(crate) measurement_kind: MeasurementKind,
    pub(crate) dimensions: TokenDimensions,
    pub(crate) cache_overlap: TokenOverlap,
    pub(crate) reasoning_overlap: TokenOverlap,
    pub(crate) normalization_version: String,
    pub(crate) source: String,
    pub(crate) source_key: String,
    pub(crate) source_revision: Option<String>,
    pub(crate) supersedes_observation_id: Option<String>,
    pub(crate) event_at: Option<String>,
    pub(crate) observed_at: String,
    pub(crate) provenance_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenUsageObservation {
    pub(crate) observation: NewUsageObservation,
    pub(crate) superseded_by_observation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletedInvocationAccounting {
    pub(crate) invocation: NewModelInvocation,
    pub(crate) observation: NewUsageObservation,
    pub(crate) status: UsageStatus,
    pub(crate) completed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageCursor {
    pub(crate) source_id: String,
    pub(crate) provider_session_id: String,
    pub(crate) epoch: u64,
    pub(crate) dimensions: TokenDimensions,
    pub(crate) ordering_key: String,
    pub(crate) source_revision: Option<String>,
    pub(crate) revision: u64,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageCursorAdvance {
    pub(crate) previous: Option<UsageCursor>,
    pub(crate) current: UsageCursor,
    pub(crate) observation: Option<NewUsageObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InvocationDetailQuery {
    pub(crate) session_id: Option<String>,
    pub(crate) agent_id: Option<String>,
    pub(crate) provider_id: Option<String>,
    pub(crate) model_id: Option<String>,
    pub(crate) purpose: Option<UsagePurpose>,
    pub(crate) quality: Option<MeasurementQuality>,
    pub(crate) status: Option<UsageStatus>,
    pub(crate) after_id: Option<String>,
    pub(crate) limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageDetailPage {
    pub(crate) invocations: Vec<ModelInvocationRecord>,
    pub(crate) observations: Vec<TokenUsageObservation>,
    pub(crate) next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageSummaryQuery {
    pub(crate) session_id: Option<String>,
    pub(crate) message_id: Option<String>,
    pub(crate) generation_id: Option<String>,
    pub(crate) agent_id: Option<String>,
    pub(crate) provider_id: Option<String>,
    pub(crate) model_id: Option<String>,
    pub(crate) purpose: Option<UsagePurpose>,
    pub(crate) quality: Option<MeasurementQuality>,
    pub(crate) status: Option<UsageStatus>,
    pub(crate) range_start: Option<String>,
    pub(crate) range_end: Option<String>,
    pub(crate) breakdown_limit: usize,
    pub(crate) generated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageMeasureAggregate {
    pub(crate) unit: AccountingUnit,
    pub(crate) dimensions: TokenDimensions,
    pub(crate) headline_total: Option<i64>,
    pub(crate) call_count: i64,
    pub(crate) observation_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageQualityAggregate {
    pub(crate) reported: UsageMeasureAggregate,
    pub(crate) reported_derived: UsageMeasureAggregate,
    pub(crate) estimated: UsageMeasureAggregate,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct UsageEntityCounts {
    pub(crate) calls: i64,
    pub(crate) generations: i64,
    pub(crate) sessions: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageDailyAggregate {
    pub(crate) local_date: String,
    pub(crate) totals: UsageQualityAggregate,
    pub(crate) counts: UsageEntityCounts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum UsageBreakdownDimension {
    Agent,
    Provider,
    Model,
    Purpose,
    Quality,
    Status,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageBreakdownEntry {
    pub(crate) key: String,
    pub(crate) totals: UsageQualityAggregate,
    pub(crate) counts: UsageEntityCounts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageBreakdown {
    pub(crate) dimension: UsageBreakdownDimension,
    pub(crate) entries: Vec<UsageBreakdownEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageAccountingSummary {
    pub(crate) totals: UsageQualityAggregate,
    pub(crate) user_response: UsageQualityAggregate,
    pub(crate) internal: UsageQualityAggregate,
    pub(crate) counts: UsageEntityCounts,
    pub(crate) daily: Vec<UsageDailyAggregate>,
    pub(crate) breakdowns: Vec<UsageBreakdown>,
    pub(crate) generated_at: String,
}
