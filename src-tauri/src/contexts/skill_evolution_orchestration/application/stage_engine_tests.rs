use std::sync::Mutex;

use super::*;
use crate::contexts::skill_evolution_orchestration::domain::{
    EvolutionStageKind, EvolutionStageStatus, EVOLUTION_STAGE_ORDER_V1,
};

struct RecordingExecutor {
    stages: Mutex<Vec<EvolutionStageKind>>,
    stop_at: Option<EvolutionStageKind>,
}

impl EvolutionStageExecutorPort for RecordingExecutor {
    fn execute(
        &self,
        dispatch: &EvolutionStageDispatchV1,
    ) -> Result<EvolutionStageDispatchResultV1, EvolutionStageEngineError> {
        self.stages.lock().expect("stage lock").push(dispatch.stage);
        let status = if self.stop_at == Some(dispatch.stage) {
            EvolutionStageStatus::PartialBudget
        } else {
            EvolutionStageStatus::Completed
        };
        Ok(EvolutionStageDispatchResultV1 {
            status,
            processed_items: 1,
            safe_failure_code: None,
        })
    }
}

#[test]
fn engine_dispatches_the_exact_eight_stage_contract_in_order() {
    let executor = RecordingExecutor {
        stages: Mutex::new(Vec::new()),
        stop_at: None,
    };
    let report =
        EvolutionStageEngineV1::execute_from("run-one", EvolutionStageKind::Recover, 1, &executor)
            .expect("run");
    assert_eq!(
        *executor.stages.lock().expect("stage lock"),
        EVOLUTION_STAGE_ORDER_V1
    );
    assert_eq!(report.completed_stages, EVOLUTION_STAGE_ORDER_V1);
    assert_eq!(report.stopped_at, None);
}

#[test]
fn partial_stage_stops_dispatch_and_resume_starts_at_the_same_stable_stage() {
    let executor = RecordingExecutor {
        stages: Mutex::new(Vec::new()),
        stop_at: Some(EvolutionStageKind::Assess),
    };
    let report = EvolutionStageEngineV1::execute_from(
        "run-one",
        EvolutionStageKind::BuildSeeds,
        2,
        &executor,
    )
    .expect("partial run");
    assert_eq!(
        *executor.stages.lock().expect("stage lock"),
        [EvolutionStageKind::BuildSeeds, EvolutionStageKind::Assess]
    );
    assert_eq!(report.stopped_at, Some(EvolutionStageKind::Assess));
    assert_eq!(report.terminal_status, EvolutionStageStatus::PartialBudget);
}

#[test]
fn engine_rejects_unsafe_identity_and_zero_attempt_before_dispatch() {
    let executor = RecordingExecutor {
        stages: Mutex::new(Vec::new()),
        stop_at: None,
    };
    assert_eq!(
        EvolutionStageEngineV1::execute_from(
            "unsafe run",
            EvolutionStageKind::Recover,
            1,
            &executor,
        ),
        Err(EvolutionStageEngineError::InvalidRun)
    );
    assert_eq!(
        EvolutionStageEngineV1::execute_from("run-one", EvolutionStageKind::Recover, 0, &executor,),
        Err(EvolutionStageEngineError::InvalidAttempt)
    );
    assert!(executor.stages.lock().expect("stage lock").is_empty());
}
