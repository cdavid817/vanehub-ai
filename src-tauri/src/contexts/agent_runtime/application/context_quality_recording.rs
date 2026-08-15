use super::{AgentClockPort, AgentLog, AgentLogLevel, AgentLoggingPort, ContextQualityRepository};
use crate::contexts::agent_runtime::domain::ContextQualityAssessmentRecord;
use std::sync::Arc;

pub(crate) struct ContextQualityRecorder {
    repository: Arc<dyn ContextQualityRepository>,
    logging: Arc<dyn AgentLoggingPort>,
    clock: Arc<dyn AgentClockPort>,
}

impl ContextQualityRecorder {
    pub(crate) fn new(
        repository: Arc<dyn ContextQualityRepository>,
        logging: Arc<dyn AgentLoggingPort>,
        clock: Arc<dyn AgentClockPort>,
    ) -> Self {
        Self {
            repository,
            logging,
            clock,
        }
    }

    pub(crate) fn record_best_effort(
        &self,
        record: &ContextQualityAssessmentRecord,
        retention_cutoff: &str,
        hard_limit: u64,
    ) {
        if self
            .repository
            .append_and_prune(record, retention_cutoff, hard_limit)
            .is_err()
        {
            self.warn();
        }
    }

    pub(crate) fn record_with_retention_days(
        &self,
        record: &ContextQualityAssessmentRecord,
        retention_days: i64,
        hard_limit: u64,
    ) {
        let cutoff = chrono::DateTime::parse_from_rfc3339(&record.recorded_at)
            .ok()
            .and_then(|recorded_at| {
                recorded_at.checked_sub_signed(chrono::Duration::days(retention_days))
            })
            .map(|cutoff| cutoff.to_rfc3339());
        match cutoff {
            Some(cutoff) => self.record_best_effort(record, &cutoff, hard_limit),
            None => self.warn(),
        }
    }

    fn warn(&self) {
        let _ = self.logging.record(AgentLog {
            level: AgentLogLevel::Warn,
            category: "agent.context.quality.persistence".to_string(),
            message: "Context quality assessment could not be persisted or pruned.".to_string(),
            agent_id: None,
            session_id: None,
            operation_id: None,
            run_id: None,
            trace_id: None,
            span_id: None,
            occurred_at: self.clock.now(),
        });
    }
}

#[cfg(test)]
#[path = "context_quality_recording_tests.rs"]
mod tests;
