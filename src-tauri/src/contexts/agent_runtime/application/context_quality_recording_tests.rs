use super::ContextQualityRecorder;
use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentLog, AgentLoggingPort, AgentRuntimeApplicationError,
    ContextQualityRepository,
};
use crate::contexts::agent_runtime::domain::{
    ContextAssessmentMeasurementQuality, ContextAssessmentOutcome, ContextQualityAssessment,
    ContextQualityAssessmentInput, ContextQualityAssessmentPage, ContextQualityAssessmentRecord,
    ContextQualitySummary,
};
use std::sync::{Arc, Mutex};

struct FailingRepository;

impl ContextQualityRepository for FailingRepository {
    fn append_and_prune(
        &self,
        _record: &ContextQualityAssessmentRecord,
        _retention_cutoff: &str,
        _hard_limit: u64,
    ) -> Result<(), AgentRuntimeApplicationError> {
        Err(AgentRuntimeApplicationError::ContextQuality(
            "secret prompt: do not persist".to_string(),
        ))
    }

    fn list(
        &self,
        _since: &str,
        _cursor: Option<&str>,
        _limit: u32,
    ) -> Result<ContextQualityAssessmentPage, AgentRuntimeApplicationError> {
        unreachable!("recording does not query history")
    }

    fn summarize(
        &self,
        _since: &str,
    ) -> Result<ContextQualitySummary, AgentRuntimeApplicationError> {
        unreachable!("recording does not query summaries")
    }
}

#[derive(Default)]
struct RecordingLogger {
    logs: Mutex<Vec<AgentLog>>,
}

impl AgentLoggingPort for RecordingLogger {
    fn record(&self, log: AgentLog) -> Result<(), AgentRuntimeApplicationError> {
        self.logs.lock().expect("logs").push(log);
        Ok(())
    }
}

struct FixedClock;

impl AgentClockPort for FixedClock {
    fn now(&self) -> String {
        "2026-08-14T12:00:00Z".to_string()
    }
}

fn record() -> ContextQualityAssessmentRecord {
    ContextQualityAssessmentRecord {
        session_correlation: Some("session-correlation".to_string()),
        recorded_at: "2026-08-14T12:00:00Z".to_string(),
        assessment: ContextQualityAssessment::new(ContextQualityAssessmentInput {
            generation_correlation: "generation-correlation",
            decision_sequence: 1,
            outcome: ContextAssessmentOutcome::Failed,
            path: None,
            reason: None,
            trigger_source: None,
            before_characters: 100,
            after_characters: 100,
            before_tokens: None,
            after_tokens: None,
            measurement_quality: ContextAssessmentMeasurementQuality::CharactersOnly,
            invariants: None,
            context_policy_version: "policy-v1",
            optimizer_version: "optimizer-v1",
            verifier_version: "verifier-v1",
        }),
    }
}

#[test]
fn persistence_failure_is_redacted_and_does_not_escape_the_best_effort_boundary() {
    let logging = Arc::new(RecordingLogger::default());
    let recorder = ContextQualityRecorder::new(
        Arc::new(FailingRepository),
        logging.clone(),
        Arc::new(FixedClock),
    );

    recorder.record_best_effort(&record(), "2026-07-15T12:00:00Z", 10_000);

    let logs = logging.logs.lock().expect("logs");
    assert_eq!(logs.len(), 1);
    let serialized = format!("{:?}", logs[0]);
    assert!(serialized.contains("agent.context.quality.persistence"));
    assert!(!serialized.contains("secret prompt"));
    assert!(!serialized.contains("session-correlation"));
    assert!(!serialized.contains("generation-correlation"));
}
