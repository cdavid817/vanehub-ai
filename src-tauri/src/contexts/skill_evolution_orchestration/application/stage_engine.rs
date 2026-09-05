use crate::contexts::skill_evolution_orchestration::domain::{
    is_safe_identifier, EvolutionStageKind, EvolutionStageStatus, EVOLUTION_STAGE_ORDER_V1,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvolutionStageDispatchV1 {
    pub(crate) run_id: String,
    pub(crate) stage: EvolutionStageKind,
    pub(crate) attempt: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvolutionStageDispatchResultV1 {
    pub(crate) status: EvolutionStageStatus,
    pub(crate) processed_items: u32,
    pub(crate) safe_failure_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvolutionStageEngineReportV1 {
    pub(crate) completed_stages: Vec<EvolutionStageKind>,
    pub(crate) stopped_at: Option<EvolutionStageKind>,
    pub(crate) terminal_status: EvolutionStageStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvolutionStageEngineError {
    InvalidRun,
    InvalidAttempt,
    Executor,
}

pub(crate) trait EvolutionStageExecutorPort: Send + Sync {
    fn execute(
        &self,
        dispatch: &EvolutionStageDispatchV1,
    ) -> Result<EvolutionStageDispatchResultV1, EvolutionStageEngineError>;
}

pub(crate) struct EvolutionStageEngineV1;

impl EvolutionStageEngineV1 {
    pub(crate) fn execute_from(
        run_id: &str,
        start_stage: EvolutionStageKind,
        attempt: u16,
        executor: &dyn EvolutionStageExecutorPort,
    ) -> Result<EvolutionStageEngineReportV1, EvolutionStageEngineError> {
        if !is_safe_identifier(run_id, 128) {
            return Err(EvolutionStageEngineError::InvalidRun);
        }
        if attempt == 0 {
            return Err(EvolutionStageEngineError::InvalidAttempt);
        }
        let start_index = EVOLUTION_STAGE_ORDER_V1
            .iter()
            .position(|stage| *stage == start_stage)
            .ok_or(EvolutionStageEngineError::Executor)?;
        let mut completed_stages = Vec::new();
        for stage in EVOLUTION_STAGE_ORDER_V1.into_iter().skip(start_index) {
            let result = executor.execute(&EvolutionStageDispatchV1 {
                run_id: run_id.into(),
                stage,
                attempt,
            })?;
            if !safe_result(&result) {
                return Err(EvolutionStageEngineError::Executor);
            }
            match result.status {
                EvolutionStageStatus::Completed | EvolutionStageStatus::SkippedEmpty => {
                    completed_stages.push(stage);
                }
                status => {
                    return Ok(EvolutionStageEngineReportV1 {
                        completed_stages,
                        stopped_at: Some(stage),
                        terminal_status: status,
                    });
                }
            }
        }
        Ok(EvolutionStageEngineReportV1 {
            completed_stages,
            stopped_at: None,
            terminal_status: EvolutionStageStatus::Completed,
        })
    }
}

fn safe_result(result: &EvolutionStageDispatchResultV1) -> bool {
    result.safe_failure_code.as_ref().is_none_or(|code| {
        is_safe_identifier(code, 64)
            && !matches!(
                result.status,
                EvolutionStageStatus::Completed | EvolutionStageStatus::SkippedEmpty
            )
    })
}
