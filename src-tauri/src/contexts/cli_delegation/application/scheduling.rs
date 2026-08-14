use super::DelegationMode;
use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DelegationLimitProfile {
    pub(crate) global_active: usize,
    pub(crate) per_session_active: usize,
    pub(crate) attempts_per_generation: u8,
    pub(crate) global_queue: usize,
    pub(crate) maximum_queue_wait: Duration,
    pub(crate) analyze_wall_time: Duration,
    pub(crate) edit_wall_time: Duration,
    pub(crate) attempt_events: u32,
    pub(crate) transcript_summary_bytes: usize,
    pub(crate) result_bytes: usize,
}

impl DelegationLimitProfile {
    pub(crate) const HARD_CEILING: Self = Self {
        global_active: 2,
        per_session_active: 1,
        attempts_per_generation: 3,
        global_queue: 16,
        maximum_queue_wait: Duration::from_secs(10 * 60),
        analyze_wall_time: Duration::from_secs(15 * 60),
        edit_wall_time: Duration::from_secs(30 * 60),
        attempt_events: 2_048,
        transcript_summary_bytes: 256 * 1024,
        result_bytes: 1024 * 1024,
    };

    pub(crate) const fn wall_time(self, mode: DelegationMode) -> Duration {
        match mode {
            DelegationMode::Analyze => self.analyze_wall_time,
            DelegationMode::Edit => self.edit_wall_time,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationQueueSnapshot {
    pub(crate) id: String,
    pub(crate) session_id: String,
    pub(crate) generation_id: String,
    pub(crate) enqueued_at_millis: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationAdmission {
    StartNow,
    Queued,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationLimitError {
    QueueFull,
    QueueWaitExceeded,
    GenerationAttemptLimit,
    EventLimit,
    TranscriptSummaryLimit,
    DurationLimit,
    ResultSizeLimit,
    UnknownDelegation,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct DelegationObservedUsage {
    pub(crate) events: u32,
    pub(crate) transcript_summary_bytes: usize,
    pub(crate) elapsed: Duration,
    pub(crate) result_bytes: usize,
}

impl DelegationObservedUsage {
    pub(crate) fn enforce(
        self,
        mode: DelegationMode,
        limits: DelegationLimitProfile,
    ) -> Result<(), DelegationLimitError> {
        if self.events > limits.attempt_events {
            return Err(DelegationLimitError::EventLimit);
        }
        if self.transcript_summary_bytes > limits.transcript_summary_bytes {
            return Err(DelegationLimitError::TranscriptSummaryLimit);
        }
        if self.elapsed > limits.wall_time(mode) {
            return Err(DelegationLimitError::DurationLimit);
        }
        if self.result_bytes > limits.result_bytes {
            return Err(DelegationLimitError::ResultSizeLimit);
        }
        Ok(())
    }
}

pub(crate) struct DelegationScheduler {
    limits: DelegationLimitProfile,
    active: BTreeMap<String, String>,
    queued: VecDeque<DelegationQueueSnapshot>,
    generation_attempts: BTreeMap<String, u8>,
}

impl DelegationScheduler {
    pub(crate) fn new(limits: DelegationLimitProfile) -> Self {
        Self {
            limits,
            active: BTreeMap::new(),
            queued: VecDeque::new(),
            generation_attempts: BTreeMap::new(),
        }
    }

    pub(crate) fn admit(
        &mut self,
        request: DelegationQueueSnapshot,
    ) -> Result<DelegationAdmission, DelegationLimitError> {
        let attempts = self
            .generation_attempts
            .entry(request.generation_id.clone())
            .or_default();
        if *attempts >= self.limits.attempts_per_generation {
            return Err(DelegationLimitError::GenerationAttemptLimit);
        }
        *attempts = attempts.saturating_add(1);
        let session_active = self
            .active
            .values()
            .filter(|session_id| *session_id == &request.session_id)
            .count();
        if self.active.len() < self.limits.global_active
            && session_active < self.limits.per_session_active
        {
            self.active.insert(request.id, request.session_id);
            return Ok(DelegationAdmission::StartNow);
        }
        if self.queued.len() >= self.limits.global_queue {
            *attempts = attempts.saturating_sub(1);
            return Err(DelegationLimitError::QueueFull);
        }
        self.queued.push_back(request);
        Ok(DelegationAdmission::Queued)
    }

    pub(crate) fn complete(
        &mut self,
        delegation_id: &str,
        now_millis: u64,
    ) -> Result<Option<DelegationQueueSnapshot>, DelegationLimitError> {
        self.active
            .remove(delegation_id)
            .ok_or(DelegationLimitError::UnknownDelegation)?;
        while let Some(candidate) = self.queued.pop_front() {
            if now_millis.saturating_sub(candidate.enqueued_at_millis)
                > duration_millis(self.limits.maximum_queue_wait)
            {
                continue;
            }
            if self
                .active
                .values()
                .any(|session| session == &candidate.session_id)
            {
                self.queued.push_back(candidate);
                return Ok(None);
            }
            self.active
                .insert(candidate.id.clone(), candidate.session_id.clone());
            return Ok(Some(candidate));
        }
        Ok(None)
    }

    pub(crate) fn expire_queued(&mut self, now_millis: u64) -> Vec<String> {
        let mut expired = Vec::new();
        self.queued.retain(|candidate| {
            let keep = now_millis.saturating_sub(candidate.enqueued_at_millis)
                <= duration_millis(self.limits.maximum_queue_wait);
            if !keep {
                expired.push(candidate.id.clone());
            }
            keep
        });
        expired
    }
}

fn duration_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
#[path = "scheduling_tests.rs"]
mod tests;
