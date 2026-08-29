use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AssessmentFallbackCategory {
    ConsentDisabled,
    ProviderUnavailable,
    ModelTimeout,
    RateLimited,
    InvalidSchema,
    ProviderFailure,
    QueuePressure,
}

impl AssessmentFallbackCategory {
    fn as_str(self) -> &'static str {
        match self {
            Self::ConsentDisabled => "consent-disabled",
            Self::ProviderUnavailable => "provider-unavailable",
            Self::ModelTimeout => "model-timeout",
            Self::RateLimited => "rate-limited",
            Self::InvalidSchema => "invalid-schema",
            Self::ProviderFailure => "provider-failure",
            Self::QueuePressure => "queue-pressure",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AssessmentFailureCategory {
    Storage,
    DatabaseLock,
    WorkerPanic,
    Shutdown,
    InvalidWitness,
}

impl AssessmentFailureCategory {
    fn as_str(self) -> &'static str {
        match self {
            Self::Storage => "storage",
            Self::DatabaseLock => "database-lock",
            Self::WorkerPanic => "worker-panic",
            Self::Shutdown => "shutdown",
            Self::InvalidWitness => "invalid-witness",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssessmentHealthSnapshot {
    pub(crate) queue_depth: usize,
    pub(crate) stale_attempts: u64,
    pub(crate) fallback_counts: [u64; 7],
    pub(crate) model_calls: u64,
    pub(crate) model_latency_total_ms: u64,
    pub(crate) model_latency_max_ms: u64,
    pub(crate) failure_counts: [u64; 5],
}

pub(crate) struct AssessmentHealth {
    queue_depth: AtomicUsize,
    stale_attempts: AtomicU64,
    fallbacks: [AtomicU64; 7],
    model_calls: AtomicU64,
    model_latency_total_ms: AtomicU64,
    model_latency_max_ms: AtomicU64,
    failures: [AtomicU64; 5],
    logging: Arc<dyn DiagnosticLogPort>,
}

impl AssessmentHealth {
    pub(crate) fn new(logging: Arc<dyn DiagnosticLogPort>) -> Self {
        Self {
            queue_depth: AtomicUsize::new(0),
            stale_attempts: AtomicU64::new(0),
            fallbacks: std::array::from_fn(|_| AtomicU64::new(0)),
            model_calls: AtomicU64::new(0),
            model_latency_total_ms: AtomicU64::new(0),
            model_latency_max_ms: AtomicU64::new(0),
            failures: std::array::from_fn(|_| AtomicU64::new(0)),
            logging,
        }
    }

    pub(crate) fn set_queue_depth(&self, depth: usize) {
        self.queue_depth.store(depth, Ordering::Relaxed);
    }

    pub(crate) fn recovered_stale(&self, count: usize) {
        let total = self
            .stale_attempts
            .fetch_add(count as u64, Ordering::Relaxed)
            + count as u64;
        self.emit_rate_limited(LogSeverity::Info, "stale-attempt-recovered", total, None);
    }

    pub(crate) fn fallback(&self, category: AssessmentFallbackCategory) {
        let count = self.fallbacks[category as usize].fetch_add(1, Ordering::Relaxed) + 1;
        self.emit_rate_limited(LogSeverity::Warn, category.as_str(), count, None);
    }

    pub(crate) fn model_latency(&self, latency_ms: u64) {
        let bounded = latency_ms.min(300_000);
        let count = self.model_calls.fetch_add(1, Ordering::Relaxed) + 1;
        self.model_latency_total_ms
            .fetch_add(bounded, Ordering::Relaxed);
        self.model_latency_max_ms
            .fetch_max(bounded, Ordering::Relaxed);
        self.emit_rate_limited(LogSeverity::Debug, "model-latency", count, Some(bounded));
    }

    pub(crate) fn failed(&self, category: AssessmentFailureCategory) {
        let count = self.failures[category as usize].fetch_add(1, Ordering::Relaxed) + 1;
        self.emit_rate_limited(LogSeverity::Error, category.as_str(), count, None);
    }

    pub(crate) fn snapshot(&self) -> AssessmentHealthSnapshot {
        AssessmentHealthSnapshot {
            queue_depth: self.queue_depth.load(Ordering::Relaxed),
            stale_attempts: self.stale_attempts.load(Ordering::Relaxed),
            fallback_counts: std::array::from_fn(|index| {
                self.fallbacks[index].load(Ordering::Relaxed)
            }),
            model_calls: self.model_calls.load(Ordering::Relaxed),
            model_latency_total_ms: self.model_latency_total_ms.load(Ordering::Relaxed),
            model_latency_max_ms: self.model_latency_max_ms.load(Ordering::Relaxed),
            failure_counts: std::array::from_fn(|index| {
                self.failures[index].load(Ordering::Relaxed)
            }),
        }
    }

    fn emit_rate_limited(
        &self,
        severity: LogSeverity,
        category: &'static str,
        count: u64,
        latency_ms: Option<u64>,
    ) {
        if count == 0 || !count.is_power_of_two() {
            return;
        }
        let mut context = BTreeMap::from([("count".to_string(), count.to_string())]);
        if let Some(latency_ms) = latency_ms {
            context.insert("latencyMs".to_string(), latency_ms.to_string());
        }
        let _ = self.logging.write_diagnostic(DiagnosticLog {
            severity,
            category: format!("skill-evolution.assessment.{category}"),
            message: "Skill evolution assessment runtime health event".to_string(),
            context,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::api::OperationsError;
    use std::sync::Mutex;

    #[derive(Default)]
    struct CapturingLogs(Mutex<Vec<DiagnosticLog>>);

    impl DiagnosticLogPort for CapturingLogs {
        fn write_diagnostic(&self, log: DiagnosticLog) -> Result<(), OperationsError> {
            self.0.lock().expect("logs").push(log);
            Ok(())
        }
    }

    #[test]
    fn health_counters_emit_only_bounded_sanitized_unified_diagnostics() {
        let logs = Arc::new(CapturingLogs::default());
        let health = AssessmentHealth::new(logs.clone());
        health.set_queue_depth(3);
        health.recovered_stale(2);
        health.fallback(AssessmentFallbackCategory::ModelTimeout);
        health.model_latency(u64::MAX);
        health.failed(AssessmentFailureCategory::DatabaseLock);
        let snapshot = health.snapshot();
        assert_eq!(snapshot.queue_depth, 3);
        assert_eq!(snapshot.stale_attempts, 2);
        assert_eq!(
            snapshot.fallback_counts[AssessmentFallbackCategory::ModelTimeout as usize],
            1
        );
        assert_eq!(snapshot.model_latency_max_ms, 300_000);
        assert_eq!(
            snapshot.failure_counts[AssessmentFailureCategory::DatabaseLock as usize],
            1
        );

        let diagnostics = logs.0.lock().expect("logs");
        assert_eq!(diagnostics.len(), 4);
        let serialized = format!("{diagnostics:?}");
        for prohibited in ["seed-", "witness", "prompt", "payload", "api_key"] {
            assert!(!serialized.contains(prohibited));
        }
    }
}
