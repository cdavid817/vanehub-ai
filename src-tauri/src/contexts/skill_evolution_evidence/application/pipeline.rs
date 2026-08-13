use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use crate::contexts::operations::api::DiagnosticLogPort;
use crate::contexts::skill_evolution_evidence::domain::EvidenceSourceEnvelope;

use super::{
    EnqueueOutcome, EvidenceEnvelopeProcessor, EvidencePriority, EvidencePriorityQueue,
    PipelineFailureCategory, PipelineHealth, PipelineHealthSnapshot,
};

pub(crate) struct EvidencePipeline {
    queue: Arc<EvidencePriorityQueue>,
    health: Arc<PipelineHealth>,
    worker: Option<JoinHandle<()>>,
}

impl EvidencePipeline {
    pub(crate) fn start(
        processor: Arc<dyn EvidenceEnvelopeProcessor>,
        logging: Arc<dyn DiagnosticLogPort>,
    ) -> Self {
        let queue = Arc::new(EvidencePriorityQueue::new());
        let health = Arc::new(PipelineHealth::new(logging));
        let worker_queue = Arc::clone(&queue);
        let worker_health = Arc::clone(&health);
        let worker = thread::Builder::new()
            .name("skill-evolution-evidence".to_string())
            .spawn(move || run_worker(worker_queue, worker_health, processor))
            .ok();
        if worker.is_none() {
            health.failed(PipelineFailureCategory::QueueUnavailable);
        }
        Self {
            queue,
            health,
            worker,
        }
    }

    pub(crate) fn enqueue(&self, envelope: EvidenceSourceEnvelope) -> EnqueueOutcome {
        let outcome = self.queue.enqueue(envelope);
        self.record_enqueue(outcome);
        outcome
    }

    pub(crate) fn enqueue_json(&self, serialized: &str) -> EnqueueOutcome {
        match serde_json::from_str(serialized) {
            Ok(envelope) => self.enqueue(envelope),
            Err(_) => {
                self.health
                    .failed(PipelineFailureCategory::MalformedEnvelope);
                let priority = EvidencePriority::Weak;
                self.health.dropped(priority);
                EnqueueOutcome::Dropped { priority }
            }
        }
    }

    pub(crate) fn health(&self) -> PipelineHealthSnapshot {
        self.health.snapshot(self.queue.depth())
    }

    pub(crate) fn shutdown(&mut self, drain: bool) -> usize {
        let discarded_by_priority = self.queue.close(drain);
        for (index, count) in discarded_by_priority.into_iter().enumerate() {
            for _ in 0..count {
                self.health.dropped(priority_from_index(index));
            }
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        discarded_by_priority.into_iter().sum()
    }

    fn record_enqueue(&self, outcome: EnqueueOutcome) {
        match outcome {
            EnqueueOutcome::Accepted { evicted, .. } => {
                self.health.accepted();
                if let Some(priority) = evicted {
                    self.health.dropped(priority);
                }
            }
            EnqueueOutcome::Dropped { priority } | EnqueueOutcome::Closed { priority } => {
                self.health.dropped(priority);
            }
        }
    }
}

impl Drop for EvidencePipeline {
    fn drop(&mut self) {
        let _ = self.shutdown(false);
    }
}

fn run_worker(
    queue: Arc<EvidencePriorityQueue>,
    health: Arc<PipelineHealth>,
    processor: Arc<dyn EvidenceEnvelopeProcessor>,
) {
    while let Some(item) = queue.dequeue() {
        let _priority = item.priority;
        let result = catch_unwind(AssertUnwindSafe(|| processor.process(&item.envelope)));
        match result {
            Ok(Ok(report)) => health.processed(report.persisted_signals),
            Ok(Err(category)) => health.failed(category),
            Err(_) => health.failed(PipelineFailureCategory::WorkerPanic),
        }
    }
}

fn priority_from_index(index: usize) -> EvidencePriority {
    match index {
        0 => EvidencePriority::Weak,
        1 => EvidencePriority::Normal,
        2 => EvidencePriority::Correlated,
        3 => EvidencePriority::Verified,
        _ => EvidencePriority::Critical,
    }
}

#[cfg(test)]
mod tests;
