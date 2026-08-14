use super::{AgentClockPort, AgentRuntimeApplicationError, ContextQualityRepository};
use crate::contexts::agent_runtime::domain::{ContextQualityAssessmentPage, ContextQualitySummary};
use std::sync::Arc;

pub(crate) const DEFAULT_CONTEXT_QUALITY_PAGE_SIZE: u32 = 25;
pub(crate) const MAX_CONTEXT_QUALITY_PAGE_SIZE: u32 = 100;

#[derive(Clone)]
pub(crate) struct ContextQualityQueryService {
    repository: Arc<dyn ContextQualityRepository>,
    clock: Arc<dyn AgentClockPort>,
}

impl ContextQualityQueryService {
    pub(crate) fn new(
        repository: Arc<dyn ContextQualityRepository>,
        clock: Arc<dyn AgentClockPort>,
    ) -> Self {
        Self { repository, clock }
    }

    pub(crate) fn list(
        &self,
        range_days: u32,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<ContextQualityAssessmentPage, AgentRuntimeApplicationError> {
        let since = self.since(range_days)?;
        let limit = limit.unwrap_or(DEFAULT_CONTEXT_QUALITY_PAGE_SIZE);
        if limit == 0 || limit > MAX_CONTEXT_QUALITY_PAGE_SIZE {
            return Err(validation(
                "Context quality page size must be between 1 and 100.",
            ));
        }
        if cursor.is_some_and(|value| value.trim().is_empty() || value.len() > 128) {
            return Err(validation("Context quality cursor is invalid."));
        }
        self.repository.list(&since, cursor, limit)
    }

    pub(crate) fn summarize(
        &self,
        range_days: u32,
    ) -> Result<ContextQualitySummary, AgentRuntimeApplicationError> {
        self.repository.summarize(&self.since(range_days)?)
    }

    fn since(&self, range_days: u32) -> Result<String, AgentRuntimeApplicationError> {
        if !matches!(range_days, 7 | 30 | 90) {
            return Err(validation(
                "Context quality range must be 7, 30, or 90 days.",
            ));
        }
        chrono::DateTime::parse_from_rfc3339(&self.clock.now())
            .ok()
            .and_then(|now| now.checked_sub_signed(chrono::Duration::days(i64::from(range_days))))
            .map(|since| since.to_rfc3339())
            .ok_or_else(|| validation("Context quality time range is unavailable."))
    }
}

fn validation(message: &str) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Validation(message.to_string())
}
