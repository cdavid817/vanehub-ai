use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

use crate::contexts::skill_evolution_evidence::domain::{
    EvidenceSourceEnvelope, FeedbackState, TerminalOutcome, UtilityOutcome, VerificationOutcome,
};

pub(crate) const EVIDENCE_QUEUE_CAPACITY: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum EvidencePriority {
    Weak,
    Normal,
    Correlated,
    Verified,
    Critical,
}

impl EvidencePriority {
    const DESCENDING: [Self; 5] = [
        Self::Critical,
        Self::Verified,
        Self::Correlated,
        Self::Normal,
        Self::Weak,
    ];

    const fn index(self) -> usize {
        self as usize
    }

    const fn admission_limit(self) -> usize {
        match self {
            Self::Critical => EVIDENCE_QUEUE_CAPACITY,
            Self::Verified => 480,
            Self::Correlated => 416,
            Self::Normal => 352,
            Self::Weak => 320,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EnqueueOutcome {
    Accepted {
        priority: EvidencePriority,
        evicted: Option<EvidencePriority>,
    },
    Dropped {
        priority: EvidencePriority,
    },
    Closed {
        priority: EvidencePriority,
    },
}

pub(crate) struct PrioritizedEnvelope {
    pub(crate) priority: EvidencePriority,
    pub(crate) envelope: EvidenceSourceEnvelope,
}

pub(crate) struct EvidencePriorityQueue {
    state: Mutex<QueueState>,
    available: Condvar,
}

struct QueueState {
    lanes: [VecDeque<EvidenceSourceEnvelope>; 5],
    len: usize,
    closed: bool,
}

impl EvidencePriorityQueue {
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(QueueState {
                lanes: std::array::from_fn(|_| VecDeque::new()),
                len: 0,
                closed: false,
            }),
            available: Condvar::new(),
        }
    }

    pub(crate) fn enqueue(&self, envelope: EvidenceSourceEnvelope) -> EnqueueOutcome {
        let priority = priority_for(&envelope);
        let Ok(mut state) = self.state.lock() else {
            return EnqueueOutcome::Dropped { priority };
        };
        if state.closed {
            return EnqueueOutcome::Closed { priority };
        }
        if state.len < priority.admission_limit() {
            state.lanes[priority.index()].push_back(envelope);
            state.len += 1;
            self.available.notify_one();
            return EnqueueOutcome::Accepted {
                priority,
                evicted: None,
            };
        }
        let evicted = lowest_populated_below(&state, priority);
        if let Some(evicted_priority) = evicted {
            state.lanes[evicted_priority.index()].pop_front();
            state.lanes[priority.index()].push_back(envelope);
            self.available.notify_one();
            EnqueueOutcome::Accepted {
                priority,
                evicted: Some(evicted_priority),
            }
        } else {
            EnqueueOutcome::Dropped { priority }
        }
    }

    pub(crate) fn dequeue(&self) -> Option<PrioritizedEnvelope> {
        let Ok(mut state) = self.state.lock() else {
            return None;
        };
        loop {
            if let Some(item) = pop_highest(&mut state) {
                return Some(item);
            }
            if state.closed {
                return None;
            }
            let Ok(next) = self.available.wait(state) else {
                return None;
            };
            state = next;
        }
    }

    pub(crate) fn close(&self, drain: bool) -> [usize; 5] {
        let Ok(mut state) = self.state.lock() else {
            return [0; 5];
        };
        state.closed = true;
        let discarded = if drain {
            [0; 5]
        } else {
            std::array::from_fn(|index| state.lanes[index].len())
        };
        if !drain {
            state.lanes.iter_mut().for_each(VecDeque::clear);
            state.len = 0;
        }
        self.available.notify_all();
        discarded
    }

    pub(crate) fn depth(&self) -> usize {
        self.state.lock().map_or(0, |state| state.len)
    }
}

fn pop_highest(state: &mut QueueState) -> Option<PrioritizedEnvelope> {
    for priority in EvidencePriority::DESCENDING {
        if let Some(envelope) = state.lanes[priority.index()].pop_front() {
            state.len -= 1;
            return Some(PrioritizedEnvelope { priority, envelope });
        }
    }
    None
}

fn lowest_populated_below(
    state: &QueueState,
    incoming: EvidencePriority,
) -> Option<EvidencePriority> {
    [
        EvidencePriority::Weak,
        EvidencePriority::Normal,
        EvidencePriority::Correlated,
        EvidencePriority::Verified,
    ]
    .into_iter()
    .find(|priority| *priority < incoming && !state.lanes[priority.index()].is_empty())
}

pub(crate) fn priority_for(envelope: &EvidenceSourceEnvelope) -> EvidencePriority {
    match envelope {
        EvidenceSourceEnvelope::ExplicitFeedback {
            feedback: FeedbackState::Corrected,
            ..
        }
        | EvidenceSourceEnvelope::RunVerification {
            outcome: VerificationOutcome::Passed | VerificationOutcome::Failed,
            ..
        } => EvidencePriority::Critical,
        EvidenceSourceEnvelope::NativeExecution {
            outcome: TerminalOutcome::Failed,
            ..
        }
        | EvidenceSourceEnvelope::DelegatedUtility {
            outcome: UtilityOutcome::Failed | UtilityOutcome::TimedOut | UtilityOutcome::Limited,
            ..
        } => EvidencePriority::Verified,
        EvidenceSourceEnvelope::ManagedCli {
            mount_snapshot: Some(_),
            ..
        }
        | EvidenceSourceEnvelope::InteractiveCli {
            mount_snapshot: Some(_),
            ..
        } => EvidencePriority::Correlated,
        EvidenceSourceEnvelope::ExplicitFeedback {
            feedback: FeedbackState::Helpful,
            ..
        }
        | EvidenceSourceEnvelope::NativeExecution {
            outcome: TerminalOutcome::Cancelled,
            ..
        } => EvidencePriority::Normal,
        _ => EvidencePriority::Weak,
    }
}

#[cfg(test)]
mod tests;
