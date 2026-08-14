use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};

use super::EvidencePriority;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PipelineFailureCategory {
    MalformedEnvelope,
    InvalidEnvelope,
    Sanitization,
    Storage,
    WorkerPanic,
    QueueUnavailable,
}

impl PipelineFailureCategory {
    fn as_str(self) -> &'static str {
        match self {
            Self::MalformedEnvelope => "malformed-envelope",
            Self::InvalidEnvelope => "invalid-envelope",
            Self::Sanitization => "sanitization",
            Self::Storage => "storage",
            Self::WorkerPanic => "worker-panic",
            Self::QueueUnavailable => "queue-unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PipelineHealthSnapshot {
    pub(crate) queue_depth: usize,
    pub(crate) accepted: u64,
    pub(crate) processed: u64,
    pub(crate) persisted_signals: u64,
    pub(crate) dropped_by_priority: [u64; 5],
    pub(crate) failures: [u64; 6],
}

pub(crate) struct PipelineHealth {
    accepted: AtomicU64,
    processed: AtomicU64,
    persisted_signals: AtomicU64,
    dropped: [AtomicU64; 5],
    failures: [AtomicU64; 6],
    logging: Arc<dyn DiagnosticLogPort>,
}

impl PipelineHealth {
    pub(crate) fn new(logging: Arc<dyn DiagnosticLogPort>) -> Self {
        Self {
            accepted: AtomicU64::new(0),
            processed: AtomicU64::new(0),
            persisted_signals: AtomicU64::new(0),
            dropped: std::array::from_fn(|_| AtomicU64::new(0)),
            failures: std::array::from_fn(|_| AtomicU64::new(0)),
            logging,
        }
    }

    pub(crate) fn accepted(&self) {
        self.accepted.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn processed(&self, persisted_signals: usize) {
        self.processed.fetch_add(1, Ordering::Relaxed);
        self.persisted_signals
            .fetch_add(persisted_signals as u64, Ordering::Relaxed);
    }

    pub(crate) fn dropped(&self, priority: EvidencePriority) {
        let counter = &self.dropped[priority as usize];
        let count = counter.fetch_add(1, Ordering::Relaxed) + 1;
        self.emit_rate_limited(LogSeverity::Warn, "queue-drop", count, Some(priority));
    }

    pub(crate) fn failed(&self, category: PipelineFailureCategory) {
        let counter = &self.failures[category as usize];
        let count = counter.fetch_add(1, Ordering::Relaxed) + 1;
        self.emit_rate_limited(LogSeverity::Warn, category.as_str(), count, None);
    }

    pub(crate) fn snapshot(&self, queue_depth: usize) -> PipelineHealthSnapshot {
        PipelineHealthSnapshot {
            queue_depth,
            accepted: self.accepted.load(Ordering::Relaxed),
            processed: self.processed.load(Ordering::Relaxed),
            persisted_signals: self.persisted_signals.load(Ordering::Relaxed),
            dropped_by_priority: std::array::from_fn(|index| {
                self.dropped[index].load(Ordering::Relaxed)
            }),
            failures: std::array::from_fn(|index| self.failures[index].load(Ordering::Relaxed)),
        }
    }

    fn emit_rate_limited(
        &self,
        severity: LogSeverity,
        category: &'static str,
        count: u64,
        priority: Option<EvidencePriority>,
    ) {
        if !count.is_power_of_two() {
            return;
        }
        let mut context = BTreeMap::from([("count".to_string(), count.to_string())]);
        if let Some(priority) = priority {
            context.insert(
                "priority".to_string(),
                format!("{priority:?}").to_lowercase(),
            );
        }
        let _ = self.logging.write_diagnostic(DiagnosticLog {
            severity,
            category: format!("skill-evolution.pipeline.{category}"),
            message: "Skill evolution evidence pipeline degraded safely".to_string(),
            context,
        });
    }
}
