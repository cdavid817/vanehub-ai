use std::sync::Mutex;

use super::*;

struct RecordingShutdownPort {
    steps: Mutex<Vec<EvolutionShutdownStepV1>>,
    timeout_at: Option<EvolutionShutdownStepV1>,
    fail_at: Option<EvolutionShutdownStepV1>,
}

impl EvolutionShutdownPort for RecordingShutdownPort {
    fn perform(
        &self,
        step: EvolutionShutdownStepV1,
        _deadline_at_ms: i64,
    ) -> Result<bool, EvolutionShutdownStepErrorV1> {
        self.steps.lock().expect("step lock").push(step);
        if self.fail_at == Some(step) {
            return Err(EvolutionShutdownStepErrorV1::Unavailable);
        }
        Ok(self.timeout_at != Some(step))
    }
}

#[test]
fn graceful_shutdown_runs_the_exact_safety_order() {
    let port = RecordingShutdownPort {
        steps: Mutex::new(Vec::new()),
        timeout_at: None,
        fail_at: None,
    };
    let report = EvolutionShutdownCoordinatorV1::shutdown(1_000, &port);
    assert_eq!(report.outcome, EvolutionShutdownOutcomeV1::Completed);
    assert_eq!(report.completed_steps, EVOLUTION_SHUTDOWN_ORDER_V1);
    assert_eq!(
        *port.steps.lock().expect("step lock"),
        EVOLUTION_SHUTDOWN_ORDER_V1
    );
}

#[test]
fn saga_timeout_still_persists_checkpoints_and_releases_leases() {
    let port = RecordingShutdownPort {
        steps: Mutex::new(Vec::new()),
        timeout_at: Some(EvolutionShutdownStepV1::SettleApplicationSagas),
        fail_at: None,
    };
    let report = EvolutionShutdownCoordinatorV1::shutdown(1_000, &port);
    assert_eq!(report.outcome, EvolutionShutdownOutcomeV1::TimedOut);
    assert_eq!(report.completed_steps, EVOLUTION_SHUTDOWN_ORDER_V1);
    assert!(report
        .completed_steps
        .contains(&EvolutionShutdownStepV1::PersistCheckpoints));
    assert!(report
        .completed_steps
        .contains(&EvolutionShutdownStepV1::ReleaseLeases));
}

#[test]
fn one_step_failure_is_reported_without_skipping_later_cleanup() {
    let port = RecordingShutdownPort {
        steps: Mutex::new(Vec::new()),
        timeout_at: None,
        fail_at: Some(EvolutionShutdownStepV1::StopNewStageDispatch),
    };
    let report = EvolutionShutdownCoordinatorV1::shutdown(1_000, &port);
    assert_eq!(
        report.outcome,
        EvolutionShutdownOutcomeV1::CompletedWithWarnings
    );
    assert_eq!(report.warnings.len(), 1);
    assert_eq!(
        port.steps.lock().expect("step lock").last(),
        Some(&EvolutionShutdownStepV1::ReleaseLeases)
    );
}
