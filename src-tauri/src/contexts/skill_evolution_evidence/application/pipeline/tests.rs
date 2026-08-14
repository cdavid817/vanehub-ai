use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::*;
use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, OperationsError};
use crate::contexts::skill_evolution_evidence::application::ProcessingReport;
use crate::contexts::skill_evolution_evidence::domain::{
    EnvelopeCommon, FeedbackState, SourceFidelity, EVIDENCE_ENVELOPE_SCHEMA_V1,
};

#[derive(Default)]
struct CapturingLog {
    entries: Mutex<Vec<DiagnosticLog>>,
}

impl DiagnosticLogPort for CapturingLog {
    fn write_diagnostic(&self, log: DiagnosticLog) -> Result<(), OperationsError> {
        self.entries.lock().expect("logs").push(log);
        Ok(())
    }
}

struct TestProcessor {
    calls: AtomicUsize,
    panic_first: bool,
    failure: Option<PipelineFailureCategory>,
}

impl EvidenceEnvelopeProcessor for TestProcessor {
    fn process(
        &self,
        _: &EvidenceSourceEnvelope,
    ) -> Result<ProcessingReport, PipelineFailureCategory> {
        let call = self.calls.fetch_add(1, Ordering::AcqRel);
        assert!(!(self.panic_first && call == 0), "injected worker panic");
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        Ok(ProcessingReport {
            persisted_signals: 1,
        })
    }
}

fn feedback(event: usize) -> EvidenceSourceEnvelope {
    EvidenceSourceEnvelope::ExplicitFeedback {
        schema_version: EVIDENCE_ENVELOPE_SCHEMA_V1,
        common: EnvelopeCommon {
            source_event_id: format!("pipeline-{event}"),
            occurred_at: "2026-08-13T10:00:00Z".to_string(),
            stable_agent_id: None,
            session_id: None,
            message_id: None,
            run_id: None,
            attempt_id: None,
            workspace: None,
            fidelity: SourceFidelity::Native,
            observed_skill_revisions: Vec::new(),
        },
        feedback: FeedbackState::Helpful,
        feedback_revision: event as u64,
        correction_note: None,
    }
}

fn wait_until(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !condition() && Instant::now() < deadline {
        std::thread::yield_now();
    }
    assert!(condition(), "condition timed out");
}

#[test]
fn worker_recovers_after_one_envelope_panics() {
    let processor = Arc::new(TestProcessor {
        calls: AtomicUsize::new(0),
        panic_first: true,
        failure: None,
    });
    let logging = Arc::new(CapturingLog::default());
    let mut pipeline = EvidencePipeline::start(processor.clone(), logging);
    assert!(matches!(
        pipeline.enqueue(feedback(1)),
        EnqueueOutcome::Accepted { .. }
    ));
    let _ = pipeline.enqueue(feedback(2));
    wait_until(|| processor.calls.load(Ordering::Acquire) == 2);
    let snapshot = pipeline.health();
    assert_eq!(snapshot.processed, 1);
    assert_eq!(
        snapshot.failures[PipelineFailureCategory::WorkerPanic as usize],
        1
    );
    pipeline.shutdown(true);
}

#[test]
fn draining_shutdown_processes_every_accepted_item() {
    let processor = Arc::new(TestProcessor {
        calls: AtomicUsize::new(0),
        panic_first: false,
        failure: None,
    });
    let logging = Arc::new(CapturingLog::default());
    let mut pipeline = EvidencePipeline::start(processor.clone(), logging);
    for event in 0..24 {
        let _ = pipeline.enqueue(feedback(event));
    }
    assert_eq!(pipeline.shutdown(true), 0);
    assert_eq!(processor.calls.load(Ordering::Acquire), 24);
    assert_eq!(pipeline.health().processed, 24);
}

#[test]
fn malformed_input_and_storage_failure_are_fail_open_and_safely_logged() {
    let processor = Arc::new(TestProcessor {
        calls: AtomicUsize::new(0),
        panic_first: false,
        failure: Some(PipelineFailureCategory::Storage),
    });
    let logging = Arc::new(CapturingLog::default());
    let mut pipeline = EvidencePipeline::start(processor.clone(), logging.clone());
    let malformed = r#"{"token":"must-not-reach-diagnostics"}"#;
    assert!(matches!(
        pipeline.enqueue_json(malformed),
        EnqueueOutcome::Dropped { .. }
    ));
    assert!(matches!(
        pipeline.enqueue(feedback(1)),
        EnqueueOutcome::Accepted { .. }
    ));
    wait_until(|| processor.calls.load(Ordering::Acquire) == 1);
    pipeline.shutdown(true);

    let snapshot = pipeline.health();
    assert_eq!(
        snapshot.failures[PipelineFailureCategory::MalformedEnvelope as usize],
        1
    );
    assert_eq!(
        snapshot.failures[PipelineFailureCategory::Storage as usize],
        1
    );
    let serialized = format!("{:?}", logging.entries.lock().expect("logs"));
    assert!(!serialized.contains("must-not-reach-diagnostics"));
    assert!(serialized.contains("skill-evolution.pipeline.storage"));
}
