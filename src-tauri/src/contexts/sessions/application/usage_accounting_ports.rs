#![allow(dead_code)]

use super::{
    InvocationDetailQuery, ModelInvocationRecord, NewModelInvocation, NewUsageObservation,
    SessionsApplicationError, TokenUsageObservation, UsageAccountingSummary, UsageCursor,
    UsageCursorAdvance, UsageDetailPage, UsageSummaryQuery,
};
use crate::contexts::sessions::domain::UsageStatus;

pub(crate) trait TokenAccountingRepository: Send + Sync {
    fn start_invocation(
        &self,
        invocation: &NewModelInvocation,
    ) -> Result<ModelInvocationRecord, SessionsApplicationError>;

    fn finalize_invocation(
        &self,
        invocation_id: &str,
        status: UsageStatus,
        completed_at: &str,
    ) -> Result<ModelInvocationRecord, SessionsApplicationError>;

    fn record_observation(
        &self,
        observation: &NewUsageObservation,
    ) -> Result<TokenUsageObservation, SessionsApplicationError>;

    fn advance_cursor(
        &self,
        advance: &UsageCursorAdvance,
    ) -> Result<UsageCursor, SessionsApplicationError>;

    fn find_cursor(&self, source_id: &str)
        -> Result<Option<UsageCursor>, SessionsApplicationError>;
}

pub(crate) trait TokenAccountingQueryPort: Send + Sync {
    fn usage_summary(
        &self,
        query: &UsageSummaryQuery,
    ) -> Result<UsageAccountingSummary, SessionsApplicationError>;

    fn invocation_details(
        &self,
        query: &InvocationDetailQuery,
    ) -> Result<UsageDetailPage, SessionsApplicationError>;
}

pub(crate) trait TokenAccountingPort:
    TokenAccountingRepository + TokenAccountingQueryPort
{
}

impl<T> TokenAccountingPort for T where T: TokenAccountingRepository + TokenAccountingQueryPort {}
